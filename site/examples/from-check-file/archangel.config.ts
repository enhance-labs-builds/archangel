import { defineConfig } from '@enhance-eng/archangel';

export default defineConfig({
  rules: ({ rule }) => [
    rule('filename-naming-convention', {
      files: 'src/**/*.{ts,tsx}',
      mustBeNamed: 'kebab-case',
      examples: { rejected: ['src/lib/formatDate.ts'] },
    }),

    rule('folder-naming-convention', {
      folders: ['src/**', '!src/**/__tests__'],
      mustBeNamed: 'kebab-case',
      examples: { rejected: ['src/userProfile/index.ts'] },
    }),

    rule('filename-blocklist', {
      files: '**/*.util.ts',
      mustNotExist: true,
      because: 'Name it *.helpers.ts.',
      examples: { rejected: ['src/lib/date.util.ts'] },
    }),

    rule('folder-match-with-fex', {
      files: { all: ['**/*.test.ts', { not: '**/__tests__/**' }] },
      mustNotExist: true,
      because: 'A test file goes in a __tests__ folder.',
      examples: { rejected: ['src/lib/date.test.ts'] },
    }),
  ],
});
