'use client';
import { useState } from 'react';
import { formatMoney } from '../../shared/money';

export function CartButton({ cents }: { cents: number }) {
  const [open, setOpen] = useState(false);
  return <button onClick={() => setOpen(!open)}>{formatMoney(cents)}</button>;
}
