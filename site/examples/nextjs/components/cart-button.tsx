'use client';
import { checkout } from '@/app/cart/actions';
import { formatTotal } from '@/lib/format';

export function CartButton({ total }: { total: number }) {
  return <button onClick={() => checkout()}>Pay {formatTotal(total)}</button>;
}
