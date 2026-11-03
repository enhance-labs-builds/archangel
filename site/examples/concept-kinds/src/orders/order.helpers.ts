import type { Order } from './order.entity';

export function orderTotal(order: Order) {
  return order.total;
}
