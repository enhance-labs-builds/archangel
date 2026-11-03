import { defineConfig, fsd } from '@enhance-eng/archangel';

export default defineConfig({
  extends: [fsd({ root: 'src' })],
  off: ['fsd/insignificant-slice'],
});
