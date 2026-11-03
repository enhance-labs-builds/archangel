import { saveUser } from '../db/users';
import { createUser } from '../domain/user.entity';

export async function signUp(email: string) {
  const user = createUser(email);
  await saveUser(user);
  return user;
}
