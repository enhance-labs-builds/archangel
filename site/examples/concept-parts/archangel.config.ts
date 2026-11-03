import { defineConfig } from '@enhance-eng/archangel';

export default defineConfig({
  parts: {
    feature: 'src/features/{feature}/**',
    shared: 'src/shared/**',
  },
  groups: {
    tests: '**/*.test.ts',
    client: { directive: 'use client' },
  },
  externals: {
    browser: ['react', 'react-dom'],
  },
  rules: ({ mayImport, rule }) => [
    mayImport('app', {
      feature: ['shared', 'browser'],
      shared: [],
    }),

    rule('tests-stay-out', {
      files: { not: 'tests' },
      mayNotImport: 'tests',
      because: 'Code that ships never needs a test file.',
      examples: { rejected: ['src/shared/a.ts imports src/shared/a.test.ts'] },
    }),
  ],
});
