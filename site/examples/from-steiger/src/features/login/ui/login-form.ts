import { signupLink } from '../../signup';

export function loginForm() {
  return ['email', 'password', signupLink()];
}
