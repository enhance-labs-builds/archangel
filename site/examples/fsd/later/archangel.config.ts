import { defineConfig, fsd } from '@enhance-eng/archangel';

export default defineConfig({
  extends: [fsd({ root: 'src', strict: true })],
  off: ['fsd/insignificant-slice'],
});
