import { expect, test } from 'vitest';
import { formatMoney } from './money';

test('formats cents', () => expect(formatMoney(150)).toBe('1.50'));
