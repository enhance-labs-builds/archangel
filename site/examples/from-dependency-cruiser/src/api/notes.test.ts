import { expect, test } from 'vitest';
import { listNotes } from './notes';

test('lists notes', () => expect(listNotes).toBeDefined());
