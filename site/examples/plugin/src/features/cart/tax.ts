export function tax(cents: number): number {
  return Math.round(cents * 0.1);
}
