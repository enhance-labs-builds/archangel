import { Pool } from 'pg';

export async function getHealth(pool: Pool) {
  await pool.query('select 1');
  return { ok: true };
}
