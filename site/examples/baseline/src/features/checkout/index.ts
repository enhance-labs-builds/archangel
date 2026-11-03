import { cartTotal } from '../cart';
import { currentUser } from '../account';

export function checkoutSummary(prices: number[]): string {
  return `${currentUser()}: ${cartTotal(prices)}`;
}
