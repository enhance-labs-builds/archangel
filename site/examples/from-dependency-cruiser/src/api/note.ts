import type { listNotes } from './notes';

export interface Note {
  readonly title: string;
}

export type Lister = typeof listNotes;
