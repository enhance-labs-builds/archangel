import { createElement } from 'react';
import { format } from 'date-fns';
import type { Order } from '../domain/order';

export function orderRow(order: Order) {
  return createElement('li', null, `${format(new Date(), 'PP')}: ${order.total}`);
}
