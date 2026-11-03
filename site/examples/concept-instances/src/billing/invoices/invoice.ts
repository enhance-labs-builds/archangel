import { invoiceNumber } from './number';
import { refund } from '../payments/refund';

export function invoice(total: number) {
  return { number: invoiceNumber(), total, refund };
}
