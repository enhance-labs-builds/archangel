import { Pool } from 'pg';
import type { User } from '../domain/user.entity';

const pool = new Pool();

export async function saveUser(user: User) {
  await pool.query('insert into users (email) values ($1)', [user.email]);
}
