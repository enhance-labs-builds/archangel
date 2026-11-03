import pg from 'pg';
import type { Order } from './order';

export async function saveOrder(order: Order) {
  await new pg.Pool().query('insert into orders (total) values ($1)', [order.total]);
}
