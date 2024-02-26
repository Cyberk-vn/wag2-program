import * as anchor from '@coral-xyz/anchor'
import { BN, Program, web3 } from '@coral-xyz/anchor'
import { walletAdapterIdentity } from '@metaplex-foundation/umi-signer-wallet-adapters'
import {
  MPL_TOKEN_METADATA_PROGRAM_ID,
  fetchDigitalAsset,
  findEditionMarkerFromEditionNumberPda,
  findMasterEditionPda,
  findMetadataPda,
  mplTokenMetadata,
} from '@metaplex-foundation/mpl-token-metadata'
import { Umi, publicKey } from '@metaplex-foundation/umi'
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAssociatedTokenAddress,
  getAccount,
  getMint,
  revoke,
  createAccount,
} from '@solana/spl-token'
import { WaggleMvb } from '../target/types/waggle_mvb'
import { bs58, utf8 } from '@coral-xyz/anchor/dist/cjs/utils/bytes'
import { expect, use } from 'chai'
import { dasApi } from '@metaplex-foundation/digital-asset-standard-api'
import { createUmi } from '@metaplex-foundation/umi-bundle-defaults'
import chaiAsPromised from 'chai-as-promised'
import chai from 'chai'
import { chain, last } from 'lodash'
import { createToken, delay, mintToken, mintTokenToAccount, toBN, transferTo } from './utils'
import { BalanceTree, toBytes32Array, hexToNumberArray, MerkleTree } from '../src/merkle'
import { DateTime } from 'luxon'
import { PublicKey } from '@metaplex-foundation/js'

chai.use(chaiAsPromised)

const DEFAULT_PUBLIC_KEY = '11111111111111111111111111111111'

export type MvbType = Program<WaggleMvb>
export type UserAccount = Awaited<ReturnType<MvbType['account']['userAccount']['fetch']>>
export type ReferralAccount = Awaited<ReturnType<MvbType['account']['referralAccount']['fetch']>>

const _provider = anchor.AnchorProvider.env()
let connection = _provider.connection
const confirmedConnection = new anchor.web3.Connection(connection.rpcEndpoint, {
  commitment: 'processed',
})
const provider = new anchor.AnchorProvider(confirmedConnection, _provider.wallet, {
  commitment: 'processed',
})
anchor.setProvider(provider)
const mvb: Program<WaggleMvb> = anchor.workspace.WaggleMvb as Program<WaggleMvb>
const modifyComputeUnits = web3.ComputeBudgetProgram.setComputeUnitLimit({ units: 1000000 })

const ROOT_CODE = 'MVBEE'
const REFERRAL = 'referral'

let airdropTree: BalanceTree

const _mvbErros = [...mvb.idl.errors] as const
type ErrorKeys = (typeof _mvbErros)[number]['name']
const mvbErrors = chain(_mvbErros)
  .keyBy('name')
  .mapValues((x) => x.code.toString(16))
  .value() as Record<ErrorKeys, string>

const users = Array.from({ length: 10 }, (_, index) => {
  const user = anchor.web3.Keypair.generate()
  const wallet = new anchor.Wallet(user)
  const payer = (wallet as any).payer
  const provider = new anchor.AnchorProvider(connection, wallet, {})
  const [userAccount] = anchor.web3.PublicKey.findProgramAddressSync([user.publicKey.toBuffer()], mvb.programId)
  const [droneMvbUserAccount] = anchor.web3.PublicKey.findProgramAddressSync(
    [utf8.encode('mvb'), utf8.encode('MVBD'), wallet.publicKey.toBuffer()],
    mvb.programId,
  )
  const [queenMvbUserAccount] = anchor.web3.PublicKey.findProgramAddressSync(
    [utf8.encode('mvb'), utf8.encode('MVBQ'), wallet.publicKey.toBuffer()],
    mvb.programId,
  )
  return {
    user,
    payer,
    wallet,
    provider,
    publicKey: user.publicKey,
    userAccount,
    airdropAmount: new BN(index + 1),
    droneMvbUserAccount,
    queenMvbUserAccount,
  }
})

const user1 = users[0]
const user2 = users[1]
const user3 = users[2]

const referralTree = new MerkleTree([user1].map((u) => u.publicKey.toBuffer()))

let lpMint: anchor.web3.PublicKey
let usdMint: anchor.web3.PublicKey
let wagMint: anchor.web3.PublicKey
const lpAuthority = users.pop()
let lpUsdVault: web3.PublicKey
let lpWagVault: web3.PublicKey

const serviceAuth = users.pop()

const DEFAULT_ACCOUNTS = {
  tokenProgram: TOKEN_PROGRAM_ID,
  associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
  metadataProgram: MPL_TOKEN_METADATA_PROGRAM_ID,
  systemProgram: anchor.web3.SystemProgram.programId,
  rent: anchor.web3.SYSVAR_RENT_PUBKEY,
}

type IUser = (typeof users)[0]

let umi: Umi

let statePubkey: anchor.web3.PublicKey

const honeyDrops = [new BN(2), new BN(4), new BN(6), new BN(8)] // 100
const LOCK_DURATION = new BN(2)
const DROP_CYCLE = new BN(2)
const STAKE_END = new BN(DateTime.now().plus({ seconds: 100 }).toUnixInteger())
const MINT_START = new BN(DateTime.now().plus({ seconds: 0 }).toUnixInteger())
const MINT_END = new BN(DateTime.now().plus({ seconds: 100 }).toUnixInteger())
const HARVEST_START = new BN(DateTime.now().plus({ seconds: 0 }).toUnixInteger())
const HARVEST_END = new BN(DateTime.now().plus({ seconds: 100 }).toUnixInteger())

interface IMvb {
  name: string
  symbol: string
  uri: string
  totalSupply: BN
  mint: anchor.web3.PublicKey
  mintAccount: anchor.web3.PublicKey
  mintVault: anchor.web3.PublicKey
  metadata: anchor.web3.PublicKey
  masterEdition: anchor.web3.PublicKey
  lpValuePrice: BN
  honeyDropPerCycle: BN
}

let queen: IMvb
let drone: IMvb
let collection: IMvb

const TOTAL_SUPPLY = new BN(14)
const MAX_PER_USER = new BN(6)

const INFINITE_INVITES = new BN(1000000)

const REFERRAL_LEVEL_VALUES = [toBN(0), toBN(3000, 9), toBN(5000, 9), toBN(5000, 9)]
const REFERRAL_LEVEL_MAX_INVITES = [new BN(1), new BN(3), INFINITE_INVITES, INFINITE_INVITES]

describe('solana-nft-anchor', async () => {
  before(async () => {
    umi = createUmi(provider.connection.rpcEndpoint, {
      commitment: 'processed',
    })
      .use(walletAdapterIdentity(provider.wallet))
      .use(dasApi())
      .use(mplTokenMetadata())
    ;[statePubkey] = anchor.web3.PublicKey.findProgramAddressSync([utf8.encode('state')], mvb.programId)
    queen = await createMvb({
      name: 'Queen',
      symbol: 'MVBQ',
      uri: 'https://mvb-nft-dev.waggle.network/queen-bee.json',
      totalSupply: new BN(5),
      lpValuePrice: toBN(1000, 9),
      honeyDropPerCycle: new BN(20),
    })
    drone = await createMvb({
      name: 'Drone',
      symbol: 'MVBD',
      uri: 'https://mvb-nft-dev.waggle.network/drone-bee.json',
      totalSupply: new BN(9),
      lpValuePrice: toBN(500, 9),
      honeyDropPerCycle: new BN(10),
    })
    collection = await createMvb({
      name: 'collection',
      symbol: 'WMC',
      uri: 'https://mvb-nft-dev.waggle.network/drone-bee.json',
      totalSupply: new BN(0),
      lpValuePrice: new BN(0),
      honeyDropPerCycle: new BN(0),
    })
  })
  afterEach(function () {
    const _this = this as any
    if (_this.currentTest.state === 'failed') {
      console.log(_this.currentTest.title, _this.currentTest.err)
    }
  })
  it('Fund', async () => {
    mvb.account.mvbAccount.all()
    await Promise.all(
      [...users, lpAuthority, serviceAuth].map(async ({ wallet }) => {
        const lastBlockhash = await connection.getLatestBlockhash()
        await connection.confirmTransaction({
          blockhash: lastBlockhash.blockhash,
          lastValidBlockHeight: lastBlockhash.lastValidBlockHeight,
          signature: await connection.requestAirdrop(wallet.publicKey, 10000 * web3.LAMPORTS_PER_SOL),
        })
        // const balance = await connection.getBalance(wallet.publicKey);
      }),
    )
  })
  it('Init token', async () => {
    usdMint = await createToken(provider)
    wagMint = await createToken(provider)
    lpMint = await createToken(provider, lpAuthority.publicKey)
    const lpUsd = web3.Keypair.generate()
    const lpWag = web3.Keypair.generate()
    lpUsdVault = lpUsd.publicKey
    lpWagVault = lpWag.publicKey
    await createAccount(connection, lpAuthority.payer, usdMint, lpAuthority.publicKey, lpUsd)
    await createAccount(connection, lpAuthority.payer, wagMint, lpAuthority.publicKey, lpWag)
  })
  it('Init mint', async () => {
    // => total supply LP = 1K
    await mintToken(lpAuthority.provider, lpMint, user1.publicKey, '250')
    await mintToken(lpAuthority.provider, lpMint, user2.publicKey, '250')
    await mintToken(lpAuthority.provider, lpMint, user3.publicKey, '500')

    // => total value = 20K
    await mintTokenToAccount(provider, usdMint, lpUsdVault, '10000')
    await mintTokenToAccount(provider, wagMint, lpWagVault, '100000')
  })

  it('Create State', async () => {
    await mvb.methods
      .createState(
        REFERRAL_LEVEL_VALUES,
        REFERRAL_LEVEL_MAX_INVITES,
        TOTAL_SUPPLY,
        MAX_PER_USER,
        LOCK_DURATION,
        DROP_CYCLE,
        STAKE_END,
        MINT_START,
        MINT_END,
        HARVEST_START,
        HARVEST_END,
        honeyDrops,
        new BN(20),
        new BN(10),
      )
      .accounts({
        state: statePubkey,
        service: serviceAuth.publicKey,
        authority: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        lpMint,
        lpVault: await getAssociatedTokenAddress(lpMint, statePubkey, true),
        usdMint,
        wagMint,
        lpUsdVault,
        lpWagVault,
      })
      .rpc()
    // const state = await mvb.account.stateAccount.fetch(statePubkey)
    // const rootUser = await mvb.account.userAccount.fetch(rootUserAccount)
    // const rootRefer = await mvb.account.referralAccount.fetch(rootReferralAccount)
  })
  it('Set Referral Root', async () => {
    await mvb.methods.setReferralRoot(toBytes32Array(referralTree.getRoot())).accounts({ state: statePubkey }).rpc()
  })
  it('User1 can not stake LP (not invited)', async () => {
    await expect(stakeLp(user1, toBN(100, 9))).rejectedWith(RegExp(mvbErrors['UserNotInvited']))
  })
  it('User1 apply Referral 0', async () => {
    await applyCycle0Referral(user1)
    await expect(applyCycle0Referral(user1)).rejectedWith(RegExp(mvbErrors['UserInvited']))
  })
  it('Create Referral', async () => {
    await expect(createReferralCode(user1, 'CODE')).rejectedWith(RegExp(mvbErrors['InvalidReferralCode']))
    await expect(createReferralCode(user1, 'CoDE1')).rejectedWith(RegExp(mvbErrors['InvalidReferralCode']))
    await expect(createReferralCode(user1, 'CODE&')).rejectedWith(RegExp(mvbErrors['InvalidReferralCode']))
    await expect(createReferralCode(user1, 'CODEE1')).rejectedWith(RegExp(mvbErrors['InvalidReferralCode']))
    await createReferralCode(user1, 'CODE1')
    await expect(createReferralCode(user1, 'CODE1')).rejectedWith(RegExp(/0x0/))
    await expect(createReferralCode(user1, 'CODE2')).rejectedWith(RegExp(mvbErrors['InsufficientReferral']))
  })
  it('Can not apply non-exist code', async () => {
    await expect(applyReferral(user2, 'NONEX')).rejectedWith(/0xbbf/) // non-exist
  })
  it('User2 apply referral CODE1', async () => {
    await applyReferral(user2, 'CODE1')
  })
  it('User3 can not apply referral CODE1 (already applied)', async () => {
    await expect(applyReferral(user3, 'CODE1')).rejectedWith(RegExp(mvbErrors['ReferralCodeUsed']))
  })
  it('User3 apply User2 code', async () => {
    await createReferralCode(user2, 'CODE2')
    await applyReferral(user3, 'CODE2')
  })
  it('User1 Stake LP', async () => {
    const user = user1
    const amount = toBN(100, 9)

    await stakeLp(user, amount)
    const stateLpVault = await getAssociatedTokenAddress(lpMint, statePubkey, true)
    let stateLpAcc = await getAccount(connection, stateLpVault)
    expect(stateLpAcc.amount).to.equal(BigInt(amount.toString()))

    await stakeLp(user, amount)
    stateLpAcc = await getAccount(connection, stateLpVault)
    expect(stateLpAcc.amount).to.equal(BigInt(toBN(200, 9).toString()))

    const user1Account = await mvb.account.userAccount.fetch(user.userAccount)
    expect(user1Account.lpStakedAmount.eq(toBN(200, 9))).to.true
    expect(user1Account.lpStakedValue.eq(toBN(4000, 9))).to.true
  })
  it('User2 stake LP', async () => {
    await stakeLp(user2, toBN(100, 9))
    const user1Account = await mvb.account.userAccount.fetch(user1.userAccount)
    expect(user1Account.teamStakedBonusAmount.toString()).eq(toBN(20, 9).toString())
    expect(user1Account.teamStakedBonusValue.toString()).eq(toBN(20 * 20, 9).toString())
  })
  it('User3 stake LP', async () => {
    await stakeLp(user3, toBN(100, 9))
    const user2Account = await mvb.account.userAccount.fetch(user2.userAccount)
    expect(user2Account.teamStakedAmount.toString()).eq(toBN(100, 9).toString())
    expect(user2Account.teamStakedBonusAmount.toString()).eq(toBN(20, 9).toString())

    const user1Account = await mvb.account.userAccount.fetch(user1.userAccount)
    expect(user1Account.teamStakedBonusAmount.toString()).eq(toBN(30, 9).toString())
  })
  it('Check create more referral code', async () => {
    // user2 value = 3200 => user can create 2 more referral code
    await createReferralCode(user2, 'CODE3')
    await createReferralCode(user2, 'CODE4')
    await expect(createReferralCode(user2, 'CODE5')).rejectedWith(RegExp(mvbErrors['InsufficientReferral']))
  })
  it('INFINIT referral', async () => {
    await stakeLp(user2, toBN(100, 9)) // 6000 => create max
    let user1Account = await mvb.account.userAccount.fetch(user1.userAccount)
    expect(user1Account.lpStakedValue.add(user1Account.teamStakedValue).toNumber()).greaterThanOrEqual(
      last(REFERRAL_LEVEL_VALUES).toNumber(),
    )

    const code = 'VIP01'
    await createReferralCode(user1, code)
    user1Account = await mvb.account.userAccount.fetch(user1.userAccount)
    expect(user1Account.numReferralCreated.toNumber()).eq(INFINITE_INVITES.toNumber())

    const [vipRef] = anchor.web3.PublicKey.findProgramAddressSync(
      [utf8.encode('referral'), utf8.encode(code)],
      mvb.programId,
    )
    const referralAccount = await mvb.account.referralAccount.fetch(vipRef)

    await applyReferral(users[3], code)
    await applyReferral(users[4], code)
    await applyReferral(users[5], code)
  })
  it('Try unstake before lock duration', async () => {
    await stakeLp(user1, toBN(50, 9))
    await expect(unstake(user1)).rejectedWith(RegExp(mvbErrors['UnderLock']))
    await delay(3000)
    await unstake(user1)
    const userAccount = await mvb.account.userAccount.fetch(user1.userAccount)
    expect(userAccount.lpStakedAmount.eq(toBN(0, 9))).to.true
  })
  it('Create Collection', async () => {
    await mvb.methods
      .createCollection(collection.name, collection.symbol, collection.uri)
      .accounts({
        authority: provider.wallet.publicKey,
        state: statePubkey,
        collectionMint: collection.mint,
        collectionMetadata: collection.metadata,
        collectionMaster: collection.masterEdition,
        collectionVault: collection.mintVault,
        ...DEFAULT_ACCOUNTS,
      })
      .rpc()
  })
  it('Mint masters', async () => {
    await mintMasterNft(drone)
    await mintMasterNft(queen)
  })
  it('Config drone airdrop', async () => {
    const configs = users.map((user) => ({ account: user.publicKey, amount: user.airdropAmount }))
    airdropTree = new BalanceTree(configs)
    await mvb.methods
      .setMvbConfig(drone.symbol, toBytes32Array(airdropTree.getRoot()), drone.honeyDropPerCycle)
      .accounts({
        authority: provider.wallet.publicKey,
        state: statePubkey,
        mvbAccount: drone.mintAccount,
        ...DEFAULT_ACCOUNTS,
      })
      .rpc()

    const { newMint, edition, newMintMetadata, newMintEdition } = await mintCopyOfMasterByAirdrop(user1, drone)
    expect((await mvb.account.mvbUserAccount.fetch(user1.droneMvbUserAccount)).numMinted.toNumber()).eq(1)
    await expect(mintCopyOfMasterByAirdrop(user1, drone)).rejectedWith(RegExp(mvbErrors['ExceededUserAirdrop']))
    await transferTo(user1.provider, newMint, user2.publicKey, '1')

    const user = user2
    await mintCopyOfMasterByAirdrop(user2, drone)
    expect((await mvb.account.mvbUserAccount.fetch(user2.droneMvbUserAccount)).numMinted.toNumber()).eq(1)
    await lockNft(user, drone, newMint)

    await delay(4000)

    await expect(transferTo(user2.provider, newMint, user1.publicKey, '1')).not.fulfilled

    await unlockNft(user, drone, newMint)
    const { lockAccount } = await lockNft(user, drone, newMint)

    // await transferTo(user2.provider, newMint, user1.publicKey, '1')
    // console.log(
    //   'remains=',
    //   (await mvb.account.stateAccount.fetch(statePubkey)).honeyDropRemains.map((x) => x.toString()).join(','),
    // )
    // console.log(
    //   'userDrops=',
    //   (await mvb.account.userAccount.fetch(user.mvbUserAccount)).honeyDrops.map((x) => x.toString()).join(','),
    // )
    await harvest(user, drone, newMint)
    // console.log(
    //   'remains=',
    //   (await mvb.account.stateAccount.fetch(statePubkey)).honeyDropRemains.map((x) => x.toString()).join(','),
    // )
    // console.log(
    //   'userDrops=',
    //   (await mvb.account.userAccount.fetch(user.userAccount)).honeyDrops.map((x) => x.toString()).join(','),
    // )
    await harvest(user, drone, newMint)
    // await harvest(user, drone, newMint)
    // console.log('userDuration=', (await mvb.account.nftLockAccount.fetch(lockAccount)).lockedDuration.toString())
    // console.log(
    //   'remains=',
    //   (await mvb.account.stateAccount.fetch(statePubkey)).honeyDropRemains.map((x) => x.toString()).join(','),
    // )
    // console.log(
    //   'userDrops=',
    //   (await mvb.account.userAccount.fetch(user.userAccount)).honeyDrops.map((x) => x.toString()).join(','),
    // )

    const closeAuth = web3.Keypair.generate()
    const [mvbEdition] = anchor.web3.PublicKey.findProgramAddressSync([newMint.toBuffer()], mvb.programId)
    const cleanupTx = await mvb.methods
      .cleanup()
      .accounts({
        service: serviceAuth.publicKey,
        state: statePubkey,
        mvbEditionAccount: mvbEdition,
        closeAuthority: closeAuth.publicKey,
      })
      .transaction()
    await serviceAuth.provider.sendAndConfirm(cleanupTx, [])
    console.log(`closeBlance=${await connection.getBalance(closeAuth.publicKey)}`)
  })
  it('Config queen airdrop', async () => {
    const configs = users.map((user) => ({ account: user.publicKey, amount: user.airdropAmount }))
    airdropTree = new BalanceTree(configs)
    await mvb.methods
      .setMvbConfig(queen.symbol, toBytes32Array(airdropTree.getRoot()), queen.honeyDropPerCycle)
      .accounts({
        authority: provider.wallet.publicKey,
        state: statePubkey,
        mvbAccount: queen.mintAccount,
        ...DEFAULT_ACCOUNTS,
      })
      .rpc()

    const { newMint: newQueenCopy } = await mintCopyOfMasterByAirdrop(user1, queen)
    expect((await mvb.account.mvbUserAccount.fetch(user1.queenMvbUserAccount)).numMinted.toNumber()).eq(1)
    const x = await fetchDigitalAsset(umi, publicKey(newQueenCopy))
    // console.log(x)
  })
  it('Mint by staked LP', async () => {
    const user2Account = await mvb.account.userAccount.fetch(user2.userAccount)
    expect(user2Account.lpStakedValue.toString()).eq(toBN(4000, 9).toString())
    expect(user2Account.teamStakedBonusValue.toString()).eq(toBN(400, 9).toString())

    // 4400 => 3 queens & 2 drone
    await mintCopyOfMasterByStakedLp(user2, queen)
    await mintCopyOfMasterByStakedLp(user2, queen)
    await mintCopyOfMasterByStakedLp(user2, queen)
    await mintCopyOfMasterByStakedLp(user2, drone)
    await mintCopyOfMasterByStakedLp(user2, drone)
    expect((await mvb.account.mvbUserAccount.fetch(user2.droneMvbUserAccount)).numMinted.toNumber()).eq(3)
    await expect(mintCopyOfMasterByStakedLp(user2, queen)).rejectedWith(RegExp(mvbErrors['Insufficient']))
  })
})

async function applyReferral(user: IUser, code: string) {
  const [referralAccount] = anchor.web3.PublicKey.findProgramAddressSync(
    [utf8.encode(REFERRAL), utf8.encode(code)],
    mvb.programId,
  )
  const txs = new web3.Transaction()
  await injectInitUser(txs, user)
  txs.add(
    await mvb.methods
      .applyReferral(code)
      .accounts({
        authority: user.publicKey,
        userAccount: user.userAccount,
        referralAccount,
      })
      .transaction(),
  )
  await user.provider.sendAndConfirm(txs, [])
}

async function createReferralCode(user: IUser, code: string) {
  const [referralAccount] = anchor.web3.PublicKey.findProgramAddressSync(
    [utf8.encode(REFERRAL), utf8.encode(code)],
    mvb.programId,
  )
  const tx = await mvb.methods
    .createReferral(code)
    .accounts({
      state: statePubkey,
      referralAccount,
      authority: user.publicKey,
      userAccount: user.userAccount,
      ...DEFAULT_ACCOUNTS,
    })
    .transaction()
  await user.provider.sendAndConfirm(tx, [])
}

async function applyCycle0Referral(user: IUser) {
  const txs = new web3.Transaction()
  await injectInitUser(txs, user)
  const setInvited = await mvb.methods
    .applyCycle0Referral(
      referralTree.getProof(user.publicKey.toBuffer()).map((p) => hexToNumberArray(p.toString('hex'))),
    )
    .accounts({
      state: statePubkey,
      authority: user.publicKey,
      userAccount: user.userAccount,
    })
    .transaction()
  txs.add(setInvited)
  await user1.provider.sendAndConfirm(txs, [])
}

async function harvest(user: IUser, parent: IMvb, mint: anchor.web3.PublicKey) {
  const txs = new web3.Transaction()
  await injectInitUser(txs, user)
  const lockAccount = web3.PublicKey.findProgramAddressSync(
    [utf8.encode('lock'), user.publicKey.toBuffer(), mint.toBuffer()],
    mvb.programId,
  )[0]
  const harvestTx = await mvb.methods
    .harvest(drone.symbol)
    .accounts({
      authority: user.publicKey,
      state: statePubkey,
      mvbAccount: parent.mintAccount,
      userAccount: user.userAccount,
      lockAccount,
      mint,
      slothashes: web3.SYSVAR_SLOT_HASHES_PUBKEY,
      ...DEFAULT_ACCOUNTS,
    })
    .transaction()
  txs.add(harvestTx)
  await user.provider.sendAndConfirm(txs, [])
}

async function injectInitUser(txs: anchor.web3.Transaction, user: IUser) {
  try {
    await mvb.account.userAccount.fetch(user.userAccount)
  } catch {
    txs.add(
      await mvb.methods
        .initUser()
        .accounts({
          authority: user.wallet.publicKey,
          userAccount: user.userAccount,
          ...DEFAULT_ACCOUNTS,
        })
        .transaction(),
    )
  }
}

async function unlockNft(user: IUser, parent: IMvb, mint: anchor.web3.PublicKey) {
  const lockAccount = web3.PublicKey.findProgramAddressSync(
    [utf8.encode('lock'), user.publicKey.toBuffer(), mint.toBuffer()],
    mvb.programId,
  )[0]
  const mintMetadata = new anchor.web3.PublicKey(findMetadataPda(umi, { mint: publicKey(mint) })[0])
  const mintEdition = new anchor.web3.PublicKey(findMasterEditionPda(umi, { mint: publicKey(mint) })[0])

  const unlockNftTx = await mvb.methods
    .unlockNft(parent.symbol)
    .accounts({
      authority: user.publicKey,
      state: statePubkey,
      lockAccount,
      parentMint: parent.mint,
      parentMintMaster: parent.masterEdition,
      collectionMint: collection.mint,
      mint: mint,
      mintMetadata,
      mintEdition,
      mintVault: await getAssociatedTokenAddress(mint, user.publicKey, true),
      ...DEFAULT_ACCOUNTS,
    })
    .transaction()
  await user.provider.sendAndConfirm(unlockNftTx, [])
}

async function lockNft(user: IUser, parent: IMvb, mint: anchor.web3.PublicKey) {
  const lockAccount = web3.PublicKey.findProgramAddressSync(
    [utf8.encode('lock'), user.publicKey.toBuffer(), mint.toBuffer()],
    mvb.programId,
  )[0]
  const mintMetadata = new anchor.web3.PublicKey(findMetadataPda(umi, { mint: publicKey(mint) })[0])
  const mintEdition = new anchor.web3.PublicKey(findMasterEditionPda(umi, { mint: publicKey(mint) })[0])

  const lockNftTx = await mvb.methods
    .lockNft(parent.symbol)
    .accounts({
      authority: user.publicKey,
      state: statePubkey,
      lockAccount,
      parentMint: parent.mint,
      parentMintMaster: parent.masterEdition,
      collectionMint: collection.mint,
      mint: mint,
      mintMetadata,
      mintEdition,
      mintVault: await getAssociatedTokenAddress(mint, user.publicKey, true),
      ...DEFAULT_ACCOUNTS,
    })
    .transaction()
  await user.provider.sendAndConfirm(lockNftTx, [])
  return { lockAccount, mint, mintMetadata, mintEdition }
}

async function unstake(user: IUser) {
  const tx = await mvb.methods
    .unstakeLp()
    .accounts({
      authority: user.wallet.publicKey,
      state: statePubkey,
      userAccount: user.userAccount,
      userLpVault: await getAssociatedTokenAddress(lpMint, user.wallet.publicKey, true),
      lpMint,
      lpVault: await getAssociatedTokenAddress(lpMint, statePubkey, true),
      lpUsdVault, //: await getAssociatedTokenAddress(usdMint, lpAuthority.publicKey, true),
      lpWagVault, //: await getAssociatedTokenAddress(wagMint, lpAuthority.publicKey, true),
      usdMint,
      wagMint,
      ...DEFAULT_ACCOUNTS,
    })
    .signers([])
    .transaction()
  await user.provider.sendAndConfirm(tx, [])
}

async function stakeLp(user: IUser, amount: anchor.BN) {
  let userAccount: UserAccount
  userAccount = await mvb.account.userAccount.fetchNullable(user.userAccount)
  if (!userAccount) {
    const initUser = await mvb.methods
      .initUser()
      .accounts({
        authority: user.wallet.publicKey,
        userAccount: user.userAccount,
        ...DEFAULT_ACCOUNTS,
      })
      .transaction()
    await user.provider.sendAndConfirm(initUser, [])
    userAccount = await mvb.account.userAccount.fetch(user.userAccount)
  }
  // let referralParent1 = null
  let userAccountParent1 = null
  // let referralParent2 = null
  let userAccountParent2 = null
  if (userAccount.referrer.toString() !== DEFAULT_PUBLIC_KEY) {
    // referralParent1 = userAccount.referral
    ;[userAccountParent1] = web3.PublicKey.findProgramAddressSync([userAccount.referrer.toBuffer()], mvb.programId)
    const parent1 = await mvb.account.userAccount.fetchNullable(userAccountParent1)
    if (parent1 && parent1.referrer.toString() !== DEFAULT_PUBLIC_KEY) {
      // referralParent2 = parent1.referral
      ;[userAccountParent2] = web3.PublicKey.findProgramAddressSync([parent1.referrer.toBuffer()], mvb.programId)
    }
  }

  const userStakedAccount = web3.Keypair.generate()
  const tx = await mvb.methods
    .stakeLp(amount)
    .accounts({
      authority: user.wallet.publicKey,
      userStakedAccount: userStakedAccount.publicKey,
      state: statePubkey,
      userAccount: user.userAccount,
      userLpVault: await getAssociatedTokenAddress(lpMint, user.wallet.publicKey, true),
      lpMint,
      lpVault: await getAssociatedTokenAddress(lpMint, statePubkey, true),
      lpUsdVault, //: await getAssociatedTokenAddress(usdMint, lpAuthority.publicKey, true),
      lpWagVault, //: await getAssociatedTokenAddress(wagMint, lpAuthority.publicKey, true),
      usdMint,
      wagMint,

      userAccountParent1,
      userAccountParent2,

      ...DEFAULT_ACCOUNTS,
    })
    .signers([userStakedAccount])
    .transaction()
  await user.provider.sendAndConfirm(tx, [userStakedAccount])
}

async function mintMasterNft(master: IMvb) {
  const tx = await mvb.methods
    .mintMasterNft(master.name, master.symbol, master.uri, master.totalSupply, master.lpValuePrice)
    .accounts({
      authority: provider.wallet.publicKey,
      state: statePubkey,
      mint: master.mint,
      mvbAccount: master.mintAccount,
      mintMetadata: master.metadata,
      mintMaster: master.masterEdition,
      mintVault: master.mintVault,
      ...DEFAULT_ACCOUNTS,
    })
    .signers([])
    .transaction()
  tx.add(modifyComputeUnits)
  await provider.sendAndConfirm(tx, [])
}

async function mintCopyOfMasterByAirdrop(user: IUser, master: IMvb) {
  const airdropAmount = user.airdropAmount
  const proof = airdropTree.getProof(user.publicKey, airdropAmount)
  return await mintCopyOfMaster(user, master, {
    type: 'airdrop',
    airdropData: {
      airdropAmount,
      proof,
    },
  })
}

async function mintCopyOfMasterByStakedLp(user: IUser, master: IMvb) {
  return await mintCopyOfMaster(user, master, { type: 'stakedLp' })
}

interface IMintCopyOfMasterOption {
  type: 'airdrop' | 'stakedLp'
  airdropData?: {
    airdropAmount: BN
    proof: Buffer[]
  }
}

async function mintCopyOfMaster(user: IUser, master: IMvb, option: IMintCopyOfMasterOption) {
  const txs = new web3.Transaction()

  const mvbAccount = await mvb.account.mvbAccount.fetch(master.mintAccount)
  const edition = mvbAccount.numMinted.add(new BN(1))

  const newMint = anchor.web3.Keypair.generate()
  const newMintVault = await getAssociatedTokenAddress(newMint.publicKey, user.wallet.publicKey, true, TOKEN_PROGRAM_ID)
  const newMintMetadata = new anchor.web3.PublicKey(findMetadataPda(umi, { mint: publicKey(newMint.publicKey) })[0])
  const newMintEditionMark = new anchor.web3.PublicKey(
    findEditionMarkerFromEditionNumberPda(umi, {
      editionNumber: edition.toNumber(),
      mint: publicKey(master.mint),
    })[0],
  )
  const newMintEdition = new anchor.web3.PublicKey(findMasterEditionPda(umi, { mint: publicKey(newMint.publicKey) })[0])
  const [mvbEdition] = anchor.web3.PublicKey.findProgramAddressSync([newMint.publicKey.toBuffer()], mvb.programId)

  const [mvbUserAccount, bump1] = anchor.web3.PublicKey.findProgramAddressSync(
    [utf8.encode('mvb'), utf8.encode(master.symbol), user.wallet.publicKey.toBuffer()],
    mvb.programId,
  )
  const [userAccount] = anchor.web3.PublicKey.findProgramAddressSync([user.wallet.publicKey.toBuffer()], mvb.programId)

  let createdUser = false
  try {
    if (option.type === 'stakedLp') {
      await mvb.account.userAccount.fetch(userAccount)
      createdUser = true
    } else {
      createdUser = true
    }
  } catch {}

  const mua = await mvb.account.mvbUserAccount.fetchNullable(mvbUserAccount)
  let createdMvbUser = !!mua

  const initUserTx = await mvb.methods
    .initUser()
    .accounts({
      authority: user.wallet.publicKey,
      userAccount,
      ...DEFAULT_ACCOUNTS,
    })
    .transaction()

  const initMvbUserTx = await mvb.methods
    .initMvbUser(master.symbol)
    .accounts({
      authority: user.wallet.publicKey,
      mvbAccount: master.mintAccount,
      mvbUserAccount,
      ...DEFAULT_ACCOUNTS,
    })
    .transaction()

  const mintNft =
    option.type === 'airdrop'
      ? mvb.methods.mintNftAirdrop(
          master.symbol,
          option.airdropData.airdropAmount,
          option.airdropData.proof.map((p) => hexToNumberArray(p.toString('hex'))),
        )
      : mvb.methods.mintNftFromStakedLp(master.symbol)
  const mintTx = await mintNft
    .accounts({
      authority: user.wallet.publicKey,
      state: statePubkey,
      mint: master.mint,
      mvbAccount: master.mintAccount,
      mintVault: master.mintVault,
      mintMetadata: master.metadata,
      mintMaster: master.masterEdition,

      userAccount: option.type === 'stakedLp' ? userAccount : null,
      mvbUserAccount,

      newMint: newMint.publicKey,
      newMintVault,
      newMintMetadata,
      newMintEdition,
      newMintEditionMark,

      mvbEdition,

      ...DEFAULT_ACCOUNTS,
    })
    .signers([newMint])
    .transaction()
  mintTx.add(modifyComputeUnits)
  const addToCollectionTx = await mvb.methods
    .addToCollection()
    .accounts({
      authority: user.wallet.publicKey,
      state: statePubkey,
      newMint: newMint.publicKey,
      newMintMetadata: newMintMetadata,
      mvbEdition: mvbEdition,

      collectionMint: collection.mint,
      collectionMetadata: collection.metadata,
      collectionMaster: collection.masterEdition,

      ...DEFAULT_ACCOUNTS,
    })
    .signers([])
    .transaction()

  if (!createdUser) txs.add(initUserTx)
  if (!createdMvbUser) txs.add(initMvbUserTx)
  txs.add(mintTx)
  txs.add(addToCollectionTx)

  await user.provider.sendAndConfirm(txs, [newMint], {})

  return { newMint: newMint.publicKey, mvbEdition, newMintMetadata, newMintEdition, edition, master, user }
}

async function createMvb({ name, symbol, uri, totalSupply, lpValuePrice, honeyDropPerCycle }): Promise<IMvb> {
  const [mint] = anchor.web3.PublicKey.findProgramAddressSync([utf8.encode(symbol)], mvb.programId)
  const [mintAccount] = anchor.web3.PublicKey.findProgramAddressSync(
    [utf8.encode('mvb'), utf8.encode(symbol)],
    mvb.programId,
  )
  return {
    honeyDropPerCycle,
    name,
    symbol,
    uri,
    totalSupply,
    mint,
    mintAccount,
    lpValuePrice,
    mintVault: await getAssociatedTokenAddress(mint, statePubkey, true),
    metadata: new anchor.web3.PublicKey(findMetadataPda(umi, { mint: publicKey(mint) })[0]),
    masterEdition: new anchor.web3.PublicKey(findMasterEditionPda(umi, { mint: publicKey(mint) })[0]),
  }
}
