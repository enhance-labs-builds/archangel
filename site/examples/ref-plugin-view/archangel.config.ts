import { defineConfig } from '@enhance-eng/archangel';
import dump from './plugins/dump.ts';

export default defineConfig({
  parts: { feature: 'src/features/{feature}/**' },
  plugins: { dump },
  rules: ({ rule }) => [
    rule('show', { files: 'feature', plugin: 'dump/show', options: { max: 2 } }),
  ],
});
