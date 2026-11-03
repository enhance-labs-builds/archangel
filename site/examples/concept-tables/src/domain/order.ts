import { log } from '../log';

export interface Order {
  readonly total: number;
}

export function placeOrder(order: Order) {
  log(`order of ${order.total}`);
}
