import { type Email, parseEmail } from './email.value-object';
import { userCreated } from './user-created.event';

export interface User {
  readonly email: Email;
}

export function createUser(email: string): User {
  const user = { email: parseEmail(email) };
  userCreated(user);
  return user;
}
