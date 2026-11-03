import type { User } from './user.entity';

export function userCreated(user: User) {
  return { type: 'user-created', email: user.email };
}
