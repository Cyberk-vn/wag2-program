import { getAssociatedTokenAddress } from '@solana/spl-token'
import { CONFIG, DEFAULT_ACCOUNTS, FIXED_CONFIG, IMasterNft, getNftInfo, program } from './CONFIG'
import { web3 } from '@coral-xyz/anchor'

async function main() {
  const collection = await getNftInfo(CONFIG.COLLECTION as any)
  const tx = await program.methods
    .createCollection(collection.name, collection.symbol, collection.uri)
    .accounts({
      authority: FIXED_CONFIG.provider.wallet.publicKey,
      state: FIXED_CONFIG.state,
      collectionMint: collection.mint,
      collectionMetadata: collection.metadata,
      collectionMaster: collection.masterEdition,
      collectionVault: collection.mintVault,
      ...DEFAULT_ACCOUNTS,
    })
    .signers([])
    .transaction()
  tx.add(FIXED_CONFIG.modifyComputeUnits)
  await FIXED_CONFIG.provider.sendAndConfirm(tx, [])
}

console.log('Running client.')
main()
  .then(() => console.log('Success'))
  .catch((e) => console.error(e))
