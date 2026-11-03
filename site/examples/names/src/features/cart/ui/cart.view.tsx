import { CartRow } from './cart-row';

export function CartView({ items }: { items: string[] }) {
  return <ul>{items.map((item) => <CartRow key={item} name={item} />)}</ul>;
}
