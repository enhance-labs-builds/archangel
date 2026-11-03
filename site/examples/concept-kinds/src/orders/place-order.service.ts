import type { Order } from './order.entity';
import { saveOrder } from './order.repository';
import { orderTotal } from './order.helpers';

export function placeOrder(order: Order) {
  saveOrder({ ...order, total: orderTotal(order) });
}
