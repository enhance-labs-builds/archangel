'use server';
import { saveCart } from '@/lib/cart';

export async function checkout() {
  await saveCart();
}
