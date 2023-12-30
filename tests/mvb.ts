import * as anchor from '@coral-xyz/anchor';
import { BN, Program, web3 } from '@coral-xyz/anchor';
import { walletAdapterIdentity } from '@metaplex-foundation/umi-signer-wallet-adapters';
import { getAssociatedTokenAddress } from '@solana/spl-token';
import {
  Edition,
  MPL_TOKEN_METADATA_PROGRAM_ID,
  fetchDigitalAsset,
  findEditionMarkerFromEditionNumberPda,
  findMasterEditionPda,
  findMetadataPda,
  mplTokenMetadata,
} from '@metaplex-foundation/mpl-token-metadata';
import { Umi, publicKey } from '@metaplex-foundation/umi';
import { TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { WaggleMvb } from '../target/types/waggle_mvb';
import { utf8 } from '@coral-xyz/anchor/dist/cjs/utils/bytes';
import { assert, expect, util } from 'chai';
import { dasApi } from '@metaplex-foundation/digital-asset-standard-api';
import { createUmi } from '@metaplex-foundation/umi-bundle-defaults';
import { Metaplex } from '@metaplex-foundation/js';
import chaiAsPromised from 'chai-as-promised';
import chai from 'chai';
import { chain } from 'lodash';
import { createToken } from './utils';

chai.use(chaiAsPromised);

const _provider = anchor.AnchorProvider.env();
let connection = _provider.connection;
const confirmedConnection = new anchor.web3.Connection(connection.rpcEndpoint, {
  commitment: 'processed',
});
const provider = new anchor.AnchorProvider(confirmedConnection, _provider.wallet, {
  commitment: 'processed',
  // skipPreflight: true,
});
anchor.setProvider(provider);
const mvb: Program<WaggleMvb> = anchor.workspace.WaggleMvb as Program<WaggleMvb>;
const modifyComputeUnits = web3.ComputeBudgetProgram.setComputeUnitLimit({ units: 1000000 });

const _mvbErros = [...mvb.idl.errors] as const;
type ErrorKeys = (typeof _mvbErros)[number]['name'];
const mvbErrors = chain(_mvbErros)
  .keyBy('name')
  .mapValues((x) => x.code.toString(16))
  .value() as Record<ErrorKeys, string>;

let lpMint: anchor.web3.PublicKey;
let usdtMint: anchor.web3.PublicKey;
let wagMint: anchor.web3.PublicKey;
const lpAuthority = anchor.web3.Keypair.generate();

const users = Array.from({ length: 10 }, () => {
  const user = anchor.web3.Keypair.generate();
  const wallet = new anchor.Wallet(user);
  const provider = new anchor.AnchorProvider(connection, wallet, {});
  return { user, wallet, provider };
});

const DEFAULT_ACCOUNTS = {
  tokenProgram: TOKEN_PROGRAM_ID,
  associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
  metadataProgram: MPL_TOKEN_METADATA_PROGRAM_ID,
  systemProgram: anchor.web3.SystemProgram.programId,
  rent: anchor.web3.SYSVAR_RENT_PUBKEY,
};

type IUser = (typeof users)[0];

let umi: Umi;

let statePubkey: anchor.web3.PublicKey;

interface IMvb {
  name: string;
  symbol: string;
  uri: string;
  totalSupply: BN;
  mint: anchor.web3.PublicKey;
  mintAccount: anchor.web3.PublicKey;
  mintVault: anchor.web3.PublicKey;
  metadata: anchor.web3.PublicKey;
  masterEdition: anchor.web3.PublicKey;
}

let queen: IMvb;
let drone: IMvb;
let collection: IMvb;

describe('solana-nft-anchor', async () => {
  before(async () => {
    umi = createUmi(provider.connection.rpcEndpoint, {
      commitment: 'processed',
    })
      .use(walletAdapterIdentity(provider.wallet))
      .use(dasApi())
      .use(mplTokenMetadata());
    console.log('before', provider.connection.rpcEndpoint);
    [statePubkey] = anchor.web3.PublicKey.findProgramAddressSync([utf8.encode('state')], mvb.programId);
    queen = await createMvb({
      name: 'Queen',
      symbol: 'MVBQ',
      uri: 'https://raw.githubusercontent.com/687c/solana-nft-native-client/main/metadata.json',
      totalSupply: new BN(1),
    });
    drone = await createMvb({
      name: 'Drone',
      symbol: 'MVBD',
      uri: 'https://raw.githubusercontent.com/687c/solana-nft-native-client/main/metadata.json',
      totalSupply: new BN(5),
    });
    collection = await createMvb({
      name: 'collection',
      symbol: 'MVBC',
      uri: 'https://raw.githubusercontent.com/687c/solana-nft-native-client/main/metadata.json',
      totalSupply: new BN(0),
    });

    usdtMint = await createToken(provider);
    wagMint = await createToken(provider);
    lpMint = await createToken(provider, lpAuthority.publicKey);
  });
  afterEach(function () {
    const _this = this as any;
    if (_this.currentTest.state === 'failed') {
      console.log(_this.currentTest.title, _this.currentTest.err);
    }
  });
  it('Fund', async () => {
    await Promise.all(
      users.map(async ({ wallet }) => {
        const lastBlockhash = await connection.getLatestBlockhash();
        await connection.confirmTransaction({
          blockhash: lastBlockhash.blockhash,
          lastValidBlockHeight: lastBlockhash.lastValidBlockHeight,
          signature: await connection.requestAirdrop(wallet.publicKey, 10000 * web3.LAMPORTS_PER_SOL),
        });
        const balance = await connection.getBalance(wallet.publicKey);
      }),
    );
  });
  it('Create State', async () => {
    await mvb.methods
      .createState(new BN(6))
      .accounts({
        state: statePubkey,
        authority: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([])
      .rpc();
  });

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
      .signers([])
      .rpc();
  });
  it('Mint drone', async () => {
    await mvb.methods
      .mintMasterNft(drone.name, drone.symbol, drone.uri, drone.totalSupply)
      .accounts({
        authority: provider.wallet.publicKey,
        state: statePubkey,
        mint: drone.mint,
        mvbMintAccount: drone.mintAccount,
        mintMetadata: drone.metadata,
        mintMaster: drone.masterEdition,
        mintVault: drone.mintVault,
        ...DEFAULT_ACCOUNTS,
      })
      .signers([])
      .rpc();
  });
  it('Mint queen', async () => {
    await mvb.methods
      .mintMasterNft(queen.name, queen.symbol, queen.uri, queen.totalSupply)
      .accounts({
        authority: provider.wallet.publicKey,
        state: statePubkey,
        mint: queen.mint,
        mvbMintAccount: queen.mintAccount,
        mintMetadata: queen.metadata,
        mintMaster: queen.masterEdition,
        mintVault: queen.mintVault,
        ...DEFAULT_ACCOUNTS,
      })
      .signers([])
      .rpc();
  });

  it('Mint copy of drone', async () => {
    await mintCopyOfMaster(users[0], drone);
    await mintCopyOfMaster(users[0], queen);
    const edition3 = await mintCopyOfMaster(users[0], drone);

    await expect(mintCopyOfMaster(users[0], queen)).rejectedWith(RegExp(mvbErrors['ExceededTotalSupply']));

    const state = await mvb.account.stateAccount.fetch(statePubkey);
    const queenAcc = await mvb.account.mvbMintAccount.fetch(queen.mintAccount);
    const droneAcc = await mvb.account.mvbMintAccount.fetch(drone.mintAccount);

    expect(droneAcc.numMinted.toNumber()).to.equal(2);
    expect(queenAcc.numMinted.toNumber()).to.equal(1);
    expect(state.numMinted.toNumber()).to.equal(3);

    const e = await fetchDigitalAsset(umi, publicKey(edition3.newMint)).then((x) => x.edition as Edition);
    expect(e.edition.toString()).eq('3');
  });
});

async function mintCopyOfMaster(user: IUser, master: IMvb) {
  const txs = new web3.Transaction();

  const mvbAccount = await mvb.account.stateAccount.fetch(statePubkey);

  const edition = mvbAccount.numMinted.add(new BN(1));

  const newMint = anchor.web3.Keypair.generate();
  const newMintVault = await getAssociatedTokenAddress(
    newMint.publicKey,
    user.wallet.publicKey,
    true,
    TOKEN_PROGRAM_ID,
  );
  const newMintMetadata = new anchor.web3.PublicKey(findMetadataPda(umi, { mint: publicKey(newMint.publicKey) })[0]);
  const newMintEditionMark = new anchor.web3.PublicKey(
    findEditionMarkerFromEditionNumberPda(umi, {
      editionNumber: edition.toNumber(),
      mint: publicKey(master.mint),
    })[0],
  );
  const newMintEdition = new anchor.web3.PublicKey(
    findMasterEditionPda(umi, { mint: publicKey(newMint.publicKey) })[0],
  );
  const [mvbEdition] = anchor.web3.PublicKey.findProgramAddressSync([newMint.publicKey.toBuffer()], mvb.programId);

  const tx = await mvb.methods
    .mintNft(master.name)
    .accounts({
      authority: user.wallet.publicKey,
      state: statePubkey,
      mint: master.mint,
      mvbMintAccount: master.mintAccount,
      mintVault: master.mintVault,
      mintMetadata: master.metadata,
      mintMaster: master.masterEdition,

      newMint: newMint.publicKey,
      newMintVault,
      newMintMetadata,
      newMintEdition,
      newMintEditionMark,

      mvbEdition,

      ...DEFAULT_ACCOUNTS,
    })
    .signers([newMint])
    .transaction();
  tx.add(modifyComputeUnits);

  txs.add(tx);

  const tx2 = await mvb.methods
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
    .transaction();

  txs.add(tx2);

  await user.provider.sendAndConfirm(txs, [newMint], {});

  return { newMint: newMint.publicKey, mvbEdition, newMintMetadata, newMintEdition, edition, master, user };
}

async function createMvb({ name, symbol, uri, totalSupply }): Promise<IMvb> {
  const [mint] = anchor.web3.PublicKey.findProgramAddressSync([utf8.encode(name)], mvb.programId);
  const [mintAccount] = anchor.web3.PublicKey.findProgramAddressSync(
    [utf8.encode('mvb'), utf8.encode(name)],
    mvb.programId,
  );
  return {
    name,
    symbol,
    uri,
    totalSupply,
    mint,
    mintAccount,
    mintVault: await getAssociatedTokenAddress(mint, statePubkey, true, TOKEN_PROGRAM_ID),
    metadata: new anchor.web3.PublicKey(findMetadataPda(umi, { mint: publicKey(mint) })[0]),
    masterEdition: new anchor.web3.PublicKey(findMasterEditionPda(umi, { mint: publicKey(mint) })[0]),
  };
}
