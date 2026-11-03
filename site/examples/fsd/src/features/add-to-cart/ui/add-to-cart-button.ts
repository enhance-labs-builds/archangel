import { button } from '@/shared/ui';
import { addToCart } from '../model/cart';

export function addToCartButton(name: string) {
  return button(`Add ${name}`, () => addToCart(name));
}
