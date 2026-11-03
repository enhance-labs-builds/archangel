import { defineConfig } from '@enhance-eng/archangel';

export default defineConfig({
  parts: {
    feature: 'src/features/{feature}/**',
    shared: 'src/shared/**',
  },
  rules: ({ mayImport, rule }) => [
    mayImport('app', {
      feature: ['shared'],
      shared: [],
    }, {
      because: 'A feature changes alone. Code that two features need goes in shared.',
    }),

    rule('no-react-in-shared', {
      files: 'shared',
      mayNotImport: { packages: ['react'] },
      because: 'Shared code also runs on the server.',
      examples: {
        rejected: ['src/shared/money.ts imports react'],
      },
    }),
  ],
});
