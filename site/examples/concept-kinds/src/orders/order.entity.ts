import type { Cents } from '../shared/money';

export interface Order {
  readonly id: string;
  readonly total: Cents;
}
