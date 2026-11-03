import { CartButton } from '@/components/cart-button';
import { cartTotal } from '@/lib/cart';

export default async function CartPage() {
  const total = await cartTotal();
  return <CartButton total={total} />;
}
