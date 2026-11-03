export const currency = 'USD';

export function formatTotal(cents: number): string {
  return `${currency} ${(cents / 100).toFixed(2)}`;
}
