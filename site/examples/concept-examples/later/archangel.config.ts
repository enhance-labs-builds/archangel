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
        accepted: [{ 'src/features/search/index.ts': '', 'src/features/search/query.ts': '' }],
        rejected: ['src/features/search/query.ts'],
      },
    }),

    rule('no-react-in-shared', {
      files: 'shared',
      mayNotImport: { packages: ['react'] },
      examples: {
        accepted: ['src/shared/format.ts imports src/shared/money.ts'],
        rejected: ['src/shared/format.ts imports react'],
      },
    }),
  ],
});
