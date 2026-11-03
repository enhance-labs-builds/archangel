import { defineConfig } from '@enhance-eng/archangel';

export default defineConfig({
  parts: {
    features: 'src/features/**',
    shared: 'src/shared/**',
  },
  rules: ({ rule }) => [
    rule('no-cycles', {
      files: '**',
      noCycles: true,
      because: 'Files that import each other cannot change or load one at a time.',
      examples: {
        accepted: ['cycle/a.ts imports cycle/b.ts'],
        rejected: ['cycle/a.ts imports cycle/b.ts imports cycle/a.ts'],
      },
    }),
  ],
});
