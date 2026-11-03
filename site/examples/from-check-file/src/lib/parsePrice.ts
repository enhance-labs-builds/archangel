import { cents } from './money.util';

export const parsePrice = (text: string) => cents(Number(text));
