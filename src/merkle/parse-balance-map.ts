import { PublicKey } from '@solana/web3.js'
import BN from 'bn.js'

import { BalanceTree } from './balance-tree'

// This is the blob that gets distributed and pinned to IPFS.
// It is completely sufficient for recreating the entire merkle tree.
// Anyone can verify that all air drops are included in the tree,
// and the tree has no additional distributions.
export interface MerkleDistributorInfo {
  merkleRoot: Buffer
  tokenTotal: string
  claims: {
    [account: string]: {
      amount: BN
      proof: Buffer[]
    }
  }
}

export type NewFormat = { address: string; earnings: string }

export function parseBalanceMap(balances: NewFormat[]): MerkleDistributorInfo {
  const dataByAddress = balances.reduce<{
    [address: string]: {
      amount: BN
      flags?: { [flag: string]: boolean }
    }
  }>((memo, { address: account, earnings }) => {
    if (memo[account.toString()]) {
      throw new Error(`Duplicate address: ${account.toString()}`)
    }
    const parsedNum = new BN(earnings)
    if (parsedNum.lte(new BN(0))) {
      throw new Error(`Invalid amount for account: ${account.toString()}`)
    }

    memo[account.toString()] = {
      amount: parsedNum,
    }
    return memo
  }, {})

  const sortedAddresses = Object.keys(dataByAddress).sort()

  // construct a tree
  const tree = new BalanceTree(
    sortedAddresses.map((address) => {
      const addressData = dataByAddress[address]
      if (!addressData) {
        throw new Error(`No address data for ${address}`)
      }

      return {
        account: new PublicKey(address),
        amount: addressData.amount,
      }
    }),
  )

  // generate claims
  const claims = sortedAddresses.reduce<{
    [address: string]: {
      amount: BN
      proof: Buffer[]
      flags?: { [flag: string]: boolean }
    }
  }>((memo, address) => {
    const addressData = dataByAddress[address]
    if (!addressData) {
      throw new Error(`No address data for ${address}`)
    }
    const { amount, flags } = addressData
    memo[address] = {
      amount: amount,
      proof: tree.getProof(new PublicKey(address), amount),
      ...(flags ? { flags } : {}),
    }
    return memo
  }, {})

  const tokenTotal: BN = sortedAddresses.reduce<BN>(
    (memo, key) => memo.add(dataByAddress[key]?.amount ?? new BN(0)),
    new BN(0),
  )

  return {
    merkleRoot: tree.getRoot(),
    tokenTotal: tokenTotal.toString(),
    claims,
  }
}
