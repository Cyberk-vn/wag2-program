import { getAssociatedTokenAddress } from '@solana/spl-token'
import { CONFIG, FIXED_CONFIG, program } from './CONFIG'
import { web3 } from '@coral-xyz/anchor'

async function main() {
  const tx = await program.methods
    .createState(
      CONFIG.TOTAL_SUPPLY,
      CONFIG.MAX_PER_USER,
      CONFIG.LP_LOCK_DURATION,
      CONFIG.DROP_CYCLE,
      CONFIG.MINT_END,
      CONFIG.HARVEST_END,
      CONFIG.HONEY_DROPS,
    )
    .accounts({
      state: FIXED_CONFIG.state,
      service: CONFIG.SERVICE,
      authority: FIXED_CONFIG.wallet.publicKey,
      lpMint: CONFIG.LP_MINT,
      lpVault: await getAssociatedTokenAddress(CONFIG.LP_MINT, FIXED_CONFIG.state, true),
      usdMint: CONFIG.USD_MINT,
      wagMint: CONFIG.WAG_MINT,
      lpUsdVault: CONFIG.LP_USD_VAULT,
      lpWagVault: CONFIG.LP_WAG_VAULT,
      systemProgram: web3.SystemProgram.programId,
    })
    .transaction()
  await FIXED_CONFIG.provider.sendAndConfirm(tx, [])
}

console.log('Running client.')
main()
  .then(() => console.log('Success'))
  .catch((e) => console.error(e))
