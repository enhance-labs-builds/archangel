import { formatMoney } from '../../shared/money';
import { discount } from './discount';
import { tax } from './tax';
import { total } from './total';

export function cartTotal(prices: number[]): string {
  const sum = total(prices);
  return formatMoney(sum + tax(sum) - discount(sum));
}
