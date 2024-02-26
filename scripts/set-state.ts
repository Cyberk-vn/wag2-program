import { CONFIG, DEFAULT_ACCOUNTS, FIXED_CONFIG, IMasterNft, getNftInfo, program } from './CONFIG'

async function main() {
  await program.methods
    .setStateConfig(
      null, // totalSupply
      null, // userMaxMint
      null, // lockDuration
      CONFIG.LP_LOCK_DURATION, // honeyDropCycleDuration
      CONFIG.STAKE_END, // stakeEndTime
      CONFIG.MINT_START, // mintStartTime
      CONFIG.MINT_END, // mintEndTime
      CONFIG.HARVEST_START, // harvestStartTime
      CONFIG.HARVEST_END, // dropHarvestEndTime
      CONFIG.PARENT_1_PERCENT,
      CONFIG.PARENT_2_PERCENT,
      CONFIG.REFERRAL_LEVEL_VALUES,
      CONFIG.REFERRAL_LEVEL_MAX_INVITES,
    )
    .accounts({
      authority: FIXED_CONFIG.provider.wallet.publicKey,
      state: FIXED_CONFIG.state,
      ...DEFAULT_ACCOUNTS,
    })
    .rpc()
}

console.log('Running client.')
main()
  .then(() => console.log('Success'))
  .catch((e) => console.error(e))
