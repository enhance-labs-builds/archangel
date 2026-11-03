import { createElement } from 'react';

export function Price({ cents }: { cents: number }) {
  return createElement('span', null, (cents / 100).toFixed(2));
}
