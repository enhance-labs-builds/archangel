import { fetchCart } from '../api/fetch-cart';
import { format } from '../../../shared/format';

export const total = (cents: number[]) => format(cents.length + fetchCart().length);
