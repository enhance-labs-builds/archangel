import { createElement } from 'react';
import { formatMoney } from '../../shared/money';

export const cartView = () => createElement('p', null, formatMoney(100));
