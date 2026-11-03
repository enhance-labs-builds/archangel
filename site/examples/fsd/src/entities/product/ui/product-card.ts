import { addToCartButton } from '@/features/add-to-cart';
import { button } from '@/shared/ui';

export function productCard(name: string) {
  return [button(name, () => {}), addToCartButton(name)];
}
