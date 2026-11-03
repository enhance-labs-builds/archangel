import type { Order } from '@acme/contract';
import { Price } from '@acme/ui';

export function orderRow(order: Order) {
  return Price({ cents: order.total });
}
