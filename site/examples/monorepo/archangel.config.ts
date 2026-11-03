import { defineConfig } from '@enhance-eng/archangel';

export default defineConfig({
  parts: {
    web: 'apps/web/**',
    api: 'apps/api/**',
    ui: 'packages/ui/**',
    contract: 'packages/contract/**',
  },
  rules: ({ mayImport, rule }) => [
    mayImport('workspace', {
      web: ['ui', 'contract'],
      api: ['contract'],
      ui: [],
      contract: [],
    }, {
      because: 'An app uses packages. A package never uses an app, and the contract depends on nothing.',
    }),

    rule('declared-packages', {
      files: 'apps/*/src/**',
      mayNotImport: { packages: ['*'] },
      dependency: 'undeclared',
      because: 'Each app lists what it uses in its own package.json, so it builds alone.',
      examples: {
        rejected: [{
          'apps/web/package.json': '{ "name": "@acme/web" }',
          'apps/web/src/a.ts': "import 'react';",
        }],
      },
    }),
  ],
});
