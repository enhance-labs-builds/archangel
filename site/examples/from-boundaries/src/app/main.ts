import { cartTotal } from '../features/cart/cart-total';
import { checkoutSummary } from '../features/checkout';

export function main() {
  return [cartTotal([100]), checkoutSummary([100])];
}
