import { CONFIG, DEFAULT_ACCOUNTS, FIXED_CONFIG, IMasterNft, getNftInfo, program } from './CONFIG'
import { BN, web3 } from '@coral-xyz/anchor'
import csv from 'csvtojson'
import path from 'path'
import { BalanceTree, MerkleTree, toBytes32Array } from '../src/merkle'

async function main() {
  await setCircle0('cycle0-users.dev.csv')
}

console.log('Running client.')
main()
  .then(() => console.log('Success'))
  .catch((e) => console.error(e))

async function setCircle0(fileName) {
  let configs = await csv().fromFile(path.join(__dirname, fileName))
  configs = configs.map((config) => ({
    account: new web3.PublicKey(config.account),
  }))
  const airdropTree = new MerkleTree(configs.map((x) => x.account.toBuffer()))

  const tx = await program.methods
    .setReferralRoot(toBytes32Array(airdropTree.getRoot()))
    .accounts({
      state: FIXED_CONFIG.state,
      authority: FIXED_CONFIG.provider.wallet.publicKey,
    })
    .transaction()
  await FIXED_CONFIG.provider.sendAndConfirm(tx, [])
}
