import { cartTotal } from '../cart/cart';

export function checkoutSummary(prices: number[]): string {
  return `Total: ${cartTotal(prices)}`;
}
