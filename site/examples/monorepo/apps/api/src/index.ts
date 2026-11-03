import type { Order } from '@acme/contract';
import { Price } from '@acme/ui';
import { z } from 'zod';

const order = z.object({ id: z.string(), total: z.number() });

export function parseOrder(body: unknown): Order {
  Price({ cents: 0 });
  return order.parse(body);
}
