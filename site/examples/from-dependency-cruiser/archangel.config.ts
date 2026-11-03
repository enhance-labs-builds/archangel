import { defineConfig } from '@enhance-eng/archangel';

export default defineConfig({
  parts: {
    ui: 'src/ui/**',
    api: 'src/api/**',
    db: 'src/db/**',
  },
  groups: {
    tests: '**/*.test.ts',
  },
  rules: ({ rule }) => [
    rule('no-circular', {
      files: 'src/**',
      noCycles: true,
      imports: 'all',
      examples: { rejected: ['src/a.ts imports src/b.ts imports src/a.ts'] },
    }),

    rule('ui-not-to-db', {
      files: 'ui',
      mayNotImport: 'db',
      because: 'The UI reads data through the api folder.',
      examples: { rejected: ['src/ui/list.ts imports src/db/client.ts'] },
    }),

    rule('not-to-dev-dep', {
      files: { all: ['src/**', { not: 'tests' }] },
      mayNotImport: { packages: ['*'] },
      dependency: 'dev',
      examples: {
        rejected: [{ 'src/api/a.ts': "import 'vitest';" }],
      },
    }),
  ],
});
