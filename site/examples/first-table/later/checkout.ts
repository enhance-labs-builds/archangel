import { formatMoney } from '../../shared/money';

export function checkoutSummary(prices: number[]): string {
  return `Total: ${formatMoney(prices.reduce((sum, price) => sum + price, 0))}`;
}
