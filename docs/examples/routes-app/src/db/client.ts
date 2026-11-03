import pg from 'pg';

export const query = (table: string) => new pg.Client().query(table);
