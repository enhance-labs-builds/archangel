import { productCard } from '@/entities/product';
import { addToCartButton } from '@/features/add-to-cart';

export function homePage() {
  return [productCard('Lamp'), addToCartButton('Lamp')];
}
