import * as anchor from "@coral-xyz/anchor";
import { Program, Provider } from "@coral-xyz/anchor";
import { walletAdapterIdentity } from "@metaplex-foundation/umi-signer-wallet-adapters";
import { getAssociatedTokenAddress } from "@solana/spl-token";
import {
	MPL_TOKEN_METADATA_PROGRAM_ID,
	findMasterEditionPda,
	findMetadataPda,
	mplTokenMetadata,
} from "@metaplex-foundation/mpl-token-metadata";
import { createUmi } from "@metaplex-foundation/umi-bundle-defaults";
import { publicKey } from "@metaplex-foundation/umi";
import {
	TOKEN_PROGRAM_ID,
	ASSOCIATED_TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { WaggleMvb } from "../target/types/waggle_mvb";

const provider = anchor.AnchorProvider.env()
anchor.setProvider(provider);
const mvb: Program<WaggleMvb> = anchor.workspace.WaggleMvb as Program<WaggleMvb>;

describe("solana-nft-anchor", async () => {
	
	it('Init', async () => {
		const mint = anchor.web3.Keypair.generate();
		const associatedTokenAccount = await getAssociatedTokenAddress(mint.publicKey, provider.wallet.publicKey);
		const umi = createUmi(provider.connection.rpcEndpoint)
			.use(walletAdapterIdentity(provider.wallet))
			.use(mplTokenMetadata());
			console.log('1')
		const metadataAccount = findMetadataPda(umi, {
			mint: publicKey(mint.publicKey),
		})[0]
		console.log('2')
		const masterEditionAccount = findMasterEditionPda(umi, {
			mint: publicKey(mint.publicKey),
		})[0]
		console.log('3', {
			metadataAccount: metadataAccount.toString(),
			masterEditionAccount: masterEditionAccount.toString()
		})
		const metadata = {
			name: "MVB",
			symbol: "MVB",
			uri: "https://raw.githubusercontent.com/687c/solana-nft-native-client/main/metadata.json",
		};
		console.log('3.5', MPL_TOKEN_METADATA_PROGRAM_ID.toString())
		const tx = await mvb.methods
			.initNft(metadata.name, metadata.symbol, metadata.uri)
			.accounts({
				signer: provider.publicKey,
				mint: mint.publicKey,
				associatedTokenAccount,
				metadataAccount,
				masterEditionAccount,
				tokenProgram: TOKEN_PROGRAM_ID,
				associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
				tokenMetadataProgram: MPL_TOKEN_METADATA_PROGRAM_ID,
				systemProgram: anchor.web3.SystemProgram.programId,
				rent: anchor.web3.SYSVAR_RENT_PUBKEY,
			})
			.signers([mint])
			.transaction()
			
			await provider.sendAndConfirm(tx, [mint], {skipPreflight: true})
			console.log('4')
	})

	// Configured the client to use the devnet cluster.
	// const provider = anchor.AnchorProvider.env();
	// anchor.setProvider(provider);
	// const program = anchor.workspace
	// 	.SolanaNftAnchor as Program<WaggleMvb>;

	// const signer = provider.wallet;

	// const umi = createUmi("https://api.devnet.solana.com")
	// 	.use(walletAdapterIdentity(signer))
	// 	.use(mplTokenMetadata());

	// const mint = anchor.web3.Keypair.generate();

	// // Derive the associated token address account for the mint
	// const associatedTokenAccount = await getAssociatedTokenAddress(
	// 	mint.publicKey,
	// 	signer.publicKey
	// );

	// // derive the metadata account
	// let metadataAccount = findMetadataPda(umi, {
	// 	mint: publicKey(mint.publicKey),
	// })[0];

	// //derive the master edition pda
	// let masterEditionAccount = findMasterEditionPda(umi, {
	// 	mint: publicKey(mint.publicKey),
	// })[0];

	// const metadata = {
	// 	name: "Kobeni",
	// 	symbol: "kBN",
	// 	uri: "https://raw.githubusercontent.com/687c/solana-nft-native-client/main/metadata.json",
	// };

	// it("mints nft!", async () => {
	// 	const tx = await program.methods
	// 		.initNft(metadata.name, metadata.symbol, metadata.uri)
	// 		.accounts({
	// 			signer: provider.publicKey,
	// 			mint: mint.publicKey,
	// 			associatedTokenAccount,
	// 			metadataAccount,
	// 			masterEditionAccount,
	// 			tokenProgram: TOKEN_PROGRAM_ID,
	// 			associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
	// 			tokenMetadataProgram: MPL_TOKEN_METADATA_PROGRAM_ID,
	// 			systemProgram: anchor.web3.SystemProgram.programId,
	// 			rent: anchor.web3.SYSVAR_RENT_PUBKEY,
	// 		})
	// 		.signers([mint])
	// 		.rpc();

	// 	console.log(
	// 		`mint nft tx: https://explorer.solana.com/tx/${tx}?cluster=devnet`
	// 	);
	// 	console.log(
	// 		`minted nft: https://explorer.solana.com/address/${mint.publicKey}?cluster=devnet`
	// 	);
	// });
});