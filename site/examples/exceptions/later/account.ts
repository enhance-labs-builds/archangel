// archangel-allow app.featur: the profile reads the cart size
import { cartTotal } from '../cart';

export function currentUser(): string {
  return `Sam (${cartTotal([])})`;
}
