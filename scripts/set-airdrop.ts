import { CONFIG, DEFAULT_ACCOUNTS, FIXED_CONFIG, IMasterNft, getNftInfo, program } from './CONFIG'
import { BN, web3 } from '@coral-xyz/anchor'
import csv from 'csvtojson'
import path from 'path'
import { BalanceTree, toBytes32Array } from '../src/merkle'

async function main() {
  await setAirdrop(CONFIG.DRONE)
  await setAirdrop(CONFIG.QUEEN)
}

console.log('Running client.')
main()
  .then(() => console.log('Success'))
  .catch((e) => console.error(e))

async function setAirdrop(master: IMasterNft) {
  const nft = await getNftInfo(master)
  let configs = await csv().fromFile(path.join(__dirname, nft.airdrop))
  configs = configs.map((config) => ({
    account: new web3.PublicKey(config.account),
    amount: new BN(config.amount),
  }))
  const airdropTree = new BalanceTree(configs)

  const tx = await program.methods
    .setMvbConfig(nft.symbol, toBytes32Array(airdropTree.getRoot()), nft.honeyDropPerCycle)
    .accounts({
      authority: FIXED_CONFIG.provider.wallet.publicKey,
      state: FIXED_CONFIG.state,
      mvbAccount: nft.mintAccount,
      ...DEFAULT_ACCOUNTS,
    })
    .transaction()
  await FIXED_CONFIG.provider.sendAndConfirm(tx, [])
}
