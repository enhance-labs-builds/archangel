import { defineConfig } from '@enhance-eng/archangel';

export default defineConfig({
  parts: {
    api: 'apps/api/**',
    web: 'apps/web/**',
    extension: 'apps/extension/**',
    storybook: 'apps/storybook/**',
    cli: 'apps/cli/**',
    desktop: 'apps/desktop/**',
    docs: 'apps/docs/**',
    contract: 'packages/contract/**',
    ui: 'packages/ui/**',
    'ledger-document': 'packages/ledger-document/**',
    'ledger-format': 'packages/ledger-format/**',
    'image-editing': 'packages/image-editing/**',
    'file-uploads': 'packages/file-uploads/**',
    'report-renderer': 'packages/report-renderer/**',
    sdk: 'packages/sdk/**',
    'plugin-loader': 'packages/plugin-loader/**',
  },
  groups: {
    tests: ['**/*.{test,spec}.{ts,tsx}', '**/__tests__/**'],
    domain: 'apps/api/src/*/domain/**',
    application: 'apps/api/src/*/application/**',
  },
  externals: {
    driver: ['pg', 'drizzle-orm', '@aws-sdk/*', 'node:fs'],
  },
  rules: ({ mayImport, rule }) => [
    mayImport('workspace', {
      web:               ['contract', 'ui', 'ledger-document', 'image-editing', 'report-renderer', 'file-uploads', 'api'],
      extension:         ['contract', 'ui', 'report-renderer'],
      storybook:         ['ui'],
      cli:               ['contract', 'ledger-document', 'sdk', 'api'],
      desktop:           ['contract'],
      api:               ['contract', 'image-editing', 'ledger-format'],
      docs:              ['cli'],
      contract:          ['image-editing', 'ledger-format', 'ui'],
      'ledger-document': ['contract', 'ui'],
      'file-uploads':    ['ledger-document', 'contract'],
      sdk:               ['report-renderer', 'contract'],
      'report-renderer': ['contract'],
      'plugin-loader':   ['cli'],
      ui:                [],
      'image-editing':   [],
      'ledger-format':   [],
    }, {
      because: 'Apps share code only through packages, and a package never reaches into an app.',
    }),

    rule('frontend-api-types-only', {
      files: 'web', mayNotImport: 'api', imports: 'runtime',
      because: 'The web app types its requests from the API router. No API code ships to the browser.',
      examples: { rejected: ['apps/web/src/a.ts imports apps/api/src/app.ts'] },
    }),

    rule('domain-not-to-pool', {
      files: ['domain', 'application'], mayNotImport: 'driver',
      because: 'The domain and application layers never touch an I/O driver. A use case that needs one declares a port.',
      examples: { rejected: ['apps/api/src/order/domain/a.entity.ts imports pg'] },
    }),

    mayImport('domain', {
      entity:           ['value-object', 'domain-event', 'specification', 'policy', 'error', 'types', 'consts'],
      'aggregate-root': ['entity', 'value-object', 'domain-event', 'specification', 'policy', 'error', 'types', 'consts'],
      'value-object':   ['error', 'types', 'consts'],
      'domain-event':   ['value-object', 'types', 'consts'],
      specification:    ['entity', 'value-object', 'aggregate-root', 'types', 'consts'],
      policy:           ['entity', 'value-object', 'aggregate-root', 'specification', 'error', 'types', 'consts'],
      port:             ['entity', 'value-object', 'error', 'types', 'consts'],
      error:            ['value-object', 'entity', 'types', 'consts'],
      types:            ['consts'],
      consts:           [],
    }, {
      files: { all: ['domain', { not: 'tests' }] },
      kinds: '**/*.{kind}.ts',
      everyRowMayImport: ['apps/api/src/shared/result.ts'],
      mayNotImportItself: ['aggregate-root', 'domain-event', 'policy', 'port'],
      because: 'Each kind of domain file depends only on the kinds below it.',
    }),
  ],
});
