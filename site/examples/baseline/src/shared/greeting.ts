import { currentUser } from '../features/account';

export function greeting(): string {
  return `Hello, ${currentUser()}`;
}
