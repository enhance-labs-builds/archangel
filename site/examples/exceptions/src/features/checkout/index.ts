// archangel-allow app.feature: checkout shows the cart total until the total moves to shared
import { cartTotal } from '../cart';
// archangel-allow app.feature
import { currentUser } from '../account';

export function checkoutSummary(prices: number[]): string {
  return `${currentUser()}: ${cartTotal(prices)}`;
}
