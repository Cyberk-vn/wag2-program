import * as anchor from '@coral-xyz/anchor'
import { AnchorProvider, Program, Provider, Wallet, web3 } from '@coral-xyz/anchor'
import { BN } from 'bn.js'
import { DateTime } from 'luxon'
import { toBN } from '../tests/utils'
import { IDL, WaggleMvb } from '../target/types/waggle_mvb'
import { utf8 } from '@coral-xyz/anchor/dist/cjs/utils/bytes'
import { createUmi } from '@metaplex-foundation/umi-bundle-defaults'
import { walletAdapterIdentity } from '@metaplex-foundation/umi-signer-wallet-adapters'
import { dasApi } from '@metaplex-foundation/digital-asset-standard-api'
import {
  MPL_TOKEN_METADATA_PROGRAM_ID,
  findMasterEditionPda,
  findMetadataPda,
  mplTokenMetadata,
} from '@metaplex-foundation/mpl-token-metadata'
import { ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID, getAssociatedTokenAddress } from '@solana/spl-token'
import { publicKey } from '@metaplex-foundation/umi'

const MVB_ID = new web3.PublicKey('6a2VeLtBtKAYu5ftup9o9uLEvfySdEzBhSDukHFZaopu')
const modifyComputeUnits = web3.ComputeBudgetProgram.setComputeUnitLimit({ units: 1000000 })

const preflightCommitment = 'recent'
const connection = new web3.Connection(process.env.SOL_URL, preflightCommitment)
const wallet = Wallet.local()

const provider = new AnchorProvider(connection, wallet, {
  preflightCommitment,
  commitment: 'recent',
})

anchor.setProvider(provider)

const umi = createUmi(provider.connection.rpcEndpoint)
  .use(walletAdapterIdentity(provider.wallet))
  .use(dasApi())
  .use(mplTokenMetadata())

export const program = new Program<WaggleMvb>(IDL, MVB_ID)

const DEV_CONFIG = {
  USD_MINT: new web3.PublicKey('4WFDSbUJJBYbLkh5UCx6gj48vrPUavhRSYGBUCMWrNmt'), // 1_000_000
  WAG_MINT: new web3.PublicKey('21sXd6E1shHL1meTE6drNQi8aAeJ95sQuLZzUXrzdn7o'), //   100_000
  LP_MINT: new web3.PublicKey('FkhzENMDtKeDQJt49HoYnhS2zvRF1yEcvfgWcRQuiijJ'), //   100_000
  LP_USD_VAULT: new web3.PublicKey('CMLb1sQqrGnn9dpw69f5XNNAe2jV1tHZHDfMF8GE3Sjv'),
  LP_WAG_VAULT: new web3.PublicKey('5u9fshvmTgdqLqGRdPfdPV3oD1pKT6oKhwYEpWqzPXt9'),
  SERVICE: new web3.PublicKey('BPdZihCt3apagWTbdt3JQJb3bmFVo8JKhY3T1N9ZV2YB'),
  TOTAL_SUPPLY: new BN(6000),
  MAX_PER_USER: new BN(5),
  LP_LOCK_DURATION: new BN(60 * 5), // 2 minutes
  DROP_CYCLE: new BN(60 * 60), // 1 hour
  MINT_END: new BN(DateTime.fromISO('2024-02-01T09:08:34').toUnixInteger()),
  HARVEST_END: new BN(DateTime.fromISO('2024-02-06T09:08:34').toUnixInteger()),
  HONEY_DROPS: [new BN(2_000), new BN(12_500), new BN(200_000), new BN(150_000)],
  COLLECTION: {
    name: 'WAGGLE MVB COLLECTION',
    symbol: 'WMC',
    uri: 'https://mvb-nft-dev.waggle.network/mvb-collection.json',
    totalSupply: new BN(6000),
    lpValuePrice: toBN('100', 6),
    honeyDropPerCycle: new BN(2),
  },
  DRONE: {
    name: 'WAGGLE MVB DRONE BEE',
    symbol: 'WMDE',
    uri: 'https://mvb-nft-dev.waggle.network/drone-bee.json',
    totalSupply: new BN(5000),
    lpValuePrice: toBN('500', 6),
    honeyDropPerCycle: new BN(4),
    airdrop: 'drone-airdrop.dev.csv',
  },
  QUEEN: {
    name: 'WAGGLE MVB QUEEN BEE',
    symbol: 'WMQB',
    uri: 'https://mvb-nft-dev.waggle.network/queen-bee.json',
    totalSupply: new BN(1000),
    lpValuePrice: toBN('1000', 6),
    honeyDropPerCycle: new BN(8),
    airdrop: 'queen-airdrop.dev.csv',
  },
}
export const CONFIG = DEV_CONFIG

export type IMasterNft = typeof CONFIG.DRONE

export const getNftInfo = async (nft: IMasterNft) => {
  const [mint] = web3.PublicKey.findProgramAddressSync([Buffer.from(nft.symbol)], MVB_ID)
  const [mintAccount] = anchor.web3.PublicKey.findProgramAddressSync(
    [utf8.encode('mvb'), utf8.encode(nft.symbol)],
    MVB_ID,
  )
  return {
    ...nft,
    mint,
    mintAccount,
    mintVault: await getAssociatedTokenAddress(mint, FIXED_CONFIG.state, true),
    metadata: new anchor.web3.PublicKey(findMetadataPda(umi, { mint: publicKey(mint) })[0]),
    masterEdition: new anchor.web3.PublicKey(findMasterEditionPda(umi, { mint: publicKey(mint) })[0]),
  }
}

export const FIXED_CONFIG = {
  modifyComputeUnits,
  program,
  umi,
  wallet,
  connection,
  provider,
  state: web3.PublicKey.findProgramAddressSync([Buffer.from('state')], MVB_ID)[0],
  collection: web3.PublicKey.findProgramAddressSync([Buffer.from('WMC')], MVB_ID)[0],
}

export const DEFAULT_ACCOUNTS = {
  tokenProgram: TOKEN_PROGRAM_ID,
  associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
  metadataProgram: MPL_TOKEN_METADATA_PROGRAM_ID,
  systemProgram: anchor.web3.SystemProgram.programId,
  rent: anchor.web3.SYSVAR_RENT_PUBKEY,
}
