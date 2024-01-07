export const toBytes32Array = (b: Buffer): number[] => {
  // invariant(b.length <= 32, `invalid length ${b.length}`);
  if (b.length > 32) {
    throw new Error(`invalid length ${b.length}`);
  }

  const buf = Buffer.alloc(32);
  b.copy(buf, 32 - b.length);

  return Array.from(buf);
};

export const hexToNumberArray = (hex: string): number[] => {
  const b = Buffer.from(hex, 'hex');
  if (b.length > 32) {
    throw new Error(`invalid length ${b.length}`);
  }

  const buf = Buffer.alloc(32);
  b.copy(buf, 32 - b.length);

  return Array.from(buf);
};
