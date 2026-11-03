import { signUp } from '../services/sign-up';

export function postUser(body: { email: string }) {
  return signUp(body.email);
}
