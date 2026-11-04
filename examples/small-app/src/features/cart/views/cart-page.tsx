import { useState } from 'react';
import { fetchCart } from '../api/fetch-cart';
import { total } from '../models/cart-total';

export const CartPage = () => useState(total(fetchCart()));
