import { keep } from './keep.value-object';
import { ok } from '../../shared/result';
import { save } from '../application/save.use-case';

export const order = [keep, ok, save];
