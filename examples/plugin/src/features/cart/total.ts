import { plus } from './plus';
import { roundCents } from './round-cents';
import { tax } from './tax';

export const total = roundCents(plus(1, tax));
