import { AnchorProvider, BN, web3 } from '@coral-xyz/anchor'
import {
  getAssociatedTokenAddress,
  createMint,
  getMint,
  mintTo,
  createAssociatedTokenAccount,
  transfer,
} from '@solana/spl-token'
import { parseUnits } from 'ethers/lib/utils'

export async function createToken(provider, authority = undefined, decimals = 9) {
  if (!authority) {
    authority = provider.wallet.publicKey
  }
  const mint = await createMint(provider.connection, provider.wallet.payer, authority, null, decimals)
  return mint
}

export async function mintToken(
  provider: AnchorProvider,
  mint: web3.PublicKey,
  toWallet: web3.PublicKey,
  amount: string,
) {
  const payer = (provider.wallet as any).payer
  const token = await getMint(provider.connection, mint)
  const decimalAmount = parseUnits(amount, token.decimals).toBigInt()
  const associatedTokenAddress = await getAssociatedTokenAddress(mint, toWallet, true)

  const otaAcc = await provider.connection.getAccountInfo(associatedTokenAddress)
  if (!otaAcc) {
    await createAssociatedTokenAccount(provider.connection, payer, mint, toWallet)
  }

  await mintTo(provider.connection, payer, mint, associatedTokenAddress, provider.wallet.publicKey, decimalAmount)
}

export async function mintTokenToAccount(
  provider: AnchorProvider,
  mint: web3.PublicKey,
  toAccount: web3.PublicKey,
  amount: string,
) {
  const payer = (provider.wallet as any).payer
  const token = await getMint(provider.connection, mint)
  const decimalAmount = parseUnits(amount, token.decimals).toBigInt()

  await mintTo(provider.connection, payer, mint, toAccount, provider.wallet.publicKey, decimalAmount)
}

export async function transferTo(
  provider: AnchorProvider,
  mint: web3.PublicKey,
  toWallet: web3.PublicKey,
  amount: string,
) {
  const payer = (provider.wallet as any).payer
  const token = await getMint(provider.connection, mint)
  const fromAcc = provider.wallet.publicKey

  const decimalAmount = parseUnits(amount, token.decimals).toBigInt()
  const fromAta = await getAssociatedTokenAddress(mint, fromAcc, true)
  const toAta = await getAssociatedTokenAddress(mint, toWallet, true)

  const otaAcc = await provider.connection.getAccountInfo(toAta)
  if (!otaAcc) {
    await createAssociatedTokenAccount(provider.connection, payer, mint, toWallet)
  }
  // connection: web3.Connection, payer: web3.Signer, source: web3.PublicKey, destination: web3.PublicKey, owner: web3.PublicKey | web3.Signer, amount: number | bigint, multiSigners?: web3.Signer[], confirmOptions?: web3.ConfirmOptions, programId?: web3.PublicKey
  await transfer(provider.connection, payer, fromAta, toAta, fromAcc, decimalAmount)

  // await mintTo(provider.connection, payer, mint, associatedTokenAddress, fromWallet, amount)
}

export function toBN(val: any, decimals: number = 9) {
  const decimalAmount = parseUnits(val.toString(), decimals)
  return new BN(decimalAmount.toString())
}

export function delay(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms))
}
