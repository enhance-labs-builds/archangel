import pg from 'pg';
import type { Order } from '../domain/order';

export async function loadOrders(): Promise<Order[]> {
  return (await new pg.Pool().query('select total from orders')).rows;
}
