import { currency } from './cart';

export function formatTotal(cents: number): string {
  return `${currency} ${(cents / 100).toFixed(2)}`;
}
