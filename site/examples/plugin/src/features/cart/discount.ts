export function discount(cents: number): number {
  return cents > 10000 ? 500 : 0;
}
