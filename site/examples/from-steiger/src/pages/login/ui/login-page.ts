import { loginForm } from '../../../features/login';
import { signupLink } from '../../../features/signup';

export function loginPage() {
  return [loginForm(), signupLink()];
}
