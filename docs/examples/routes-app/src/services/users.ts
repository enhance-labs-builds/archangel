import { query } from '../db/client';

export const listUsers = () => query('users');
