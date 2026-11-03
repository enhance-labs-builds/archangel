import 'server-only';
import { readFile } from 'node:fs/promises';

export const currency = 'USD';

export async function cartTotal(): Promise<number> {
  return Number(await readFile('cart.txt', 'utf8'));
}

export async function saveCart(): Promise<void> {}
