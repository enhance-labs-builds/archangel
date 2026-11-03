import { formatMoney } from '../../shared/money';

export function cartTotal(prices: number[]): string {
  return formatMoney(prices.reduce((sum, price) => sum + price, 0));
}
