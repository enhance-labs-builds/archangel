import { raw } from '@/shared/ui/button/button';
import { User } from '@/entities/user/model/user';
import { api } from '../../../shared/api';
import { helper } from '@/features/login/lib/helper';
import { Button } from '@/shared/ui';
export const login = [raw, User, api, helper, Button];
