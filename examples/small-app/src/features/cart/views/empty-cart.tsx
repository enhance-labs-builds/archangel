import { useMemo } from 'react';
import { format } from '../../../shared/format';

export const EmptyCart = () => useMemo(() => format(0), []);
