import { defineConfig } from '@enhance-eng/archangel';

export default defineConfig({
  parts: {
    app: 'src/app/**',
    feature: 'src/features/{featureName}/**',
    shared: 'src/shared/**',
  },
  rules: ({ entry, mayImport }) => [
    mayImport('element-types', {
      app: ['feature', 'shared'],
      feature: ['shared'],
      shared: [],
    }),

    entry('feature', 'index.ts', {
      name: 'entry-point',
      examples: {
        rejected: ['src/app/main.ts imports src/features/login/form.ts'],
      },
    }),
  ],
});
