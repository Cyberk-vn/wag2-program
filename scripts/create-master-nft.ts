import { getAssociatedTokenAddress } from '@solana/spl-token'
import { CONFIG, DEFAULT_ACCOUNTS, FIXED_CONFIG, IMasterNft, getNftInfo, program } from './CONFIG'
import { web3 } from '@coral-xyz/anchor'

async function main() {
  await mintMaster(CONFIG.QUEEN)
  await mintMaster(CONFIG.DRONE)
}

console.log('Running client.')
main()
  .then(() => console.log('Success'))
  .catch((e) => console.error(e))

async function mintMaster(nft: IMasterNft) {
  const master = await getNftInfo(nft)
  const tx = await program.methods
    .mintMasterNft(master.name, master.symbol, master.uri, master.totalSupply, master.lpValuePrice)
    .accounts({
      authority: FIXED_CONFIG.provider.wallet.publicKey,
      state: FIXED_CONFIG.state,
      mint: master.mint,
      mvbAccount: master.mintAccount,
      mintMetadata: master.metadata,
      mintMaster: master.masterEdition,
      mintVault: master.mintVault,
      ...DEFAULT_ACCOUNTS,
    })
    .signers([])
    .transaction()
  tx.add(FIXED_CONFIG.modifyComputeUnits)
  await FIXED_CONFIG.provider.sendAndConfirm(tx, [])
}
