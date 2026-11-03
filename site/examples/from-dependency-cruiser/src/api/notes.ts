import { pool } from '../db/client';
import type { Note } from './note';

export async function listNotes(): Promise<Note[]> {
  return (await pool.query('select * from notes')).rows;
}
