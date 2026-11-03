import { defineConfig } from '@enhance-eng/archangel';

export default defineConfig({
  parts: {
    feature: 'src/features/{feature}/**',
    shared: 'src/shared/**',
  },
  rules: ({ rule }) => [
    rule('feature-entry', {
      folders: 'src/features/*',
      mustContain: 'index.ts',
      examples: {
        accepted: ['src/shared/format.ts'],
        rejected: ['src/features/cart/model.ts'],
      },
    }),

    rule('no-react-in-shared', {
      files: 'shared',
      mayNotImport: { packages: ['react'] },
      examples: {
        accepted: ['src/shared/format.ts imports src/shared/money.ts'],
      },
    }),
  ],
});
