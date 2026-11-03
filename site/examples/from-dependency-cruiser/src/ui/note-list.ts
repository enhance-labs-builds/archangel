import { listNotes } from '../api/notes';
import { pool } from '../db/client';
import { vi } from 'vitest';

export async function noteList() {
  vi.fn();
  await pool.query('select 1');
  return (await listNotes()).map((note) => note.title);
}
