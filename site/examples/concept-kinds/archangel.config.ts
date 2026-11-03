import { defineConfig } from '@enhance-eng/archangel';

export default defineConfig({
  parts: {
    shared: 'src/shared/**',
  },
  rules: ({ mayImport }) => [
    mayImport('orders', {
      service: ['repository', 'entity', 'shared'],
      repository: ['entity', 'shared'],
      entity: ['shared'],
      shared: [],
    }, {
      kinds: 'src/**/*.{kind}.ts',
      files: 'src/**',
      because: 'A service uses repositories. A repository stores entities. An entity knows only itself.',
    }),
  ],
});
