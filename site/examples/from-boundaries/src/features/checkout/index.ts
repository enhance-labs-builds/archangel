import { cartTotal } from '../cart';

export function checkoutSummary(prices: number[]): string {
  return `Total: ${cartTotal(prices)}`;
}
