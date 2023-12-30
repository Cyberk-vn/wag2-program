import { getAssociatedTokenAddress, createMint, TOKEN_2022_PROGRAM_ID } from '@solana/spl-token';

export async function createToken(provider, authority = undefined, decimals = 9) {
  if (!authority) {
    authority = provider.wallet.publicKey;
  }
  const mint = await createMint(
    provider.connection,
    provider.wallet.payer,
    authority,
    null,
    decimals,
    null,
    null,
    TOKEN_2022_PROGRAM_ID,
  );
  return mint;
}
