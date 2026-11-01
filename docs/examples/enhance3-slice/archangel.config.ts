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
    'canvas-document': 'packages/canvas-document/**',
    'canvas-format': 'packages/canvas-format/**',
    'artwork-paint': 'packages/artwork-paint/**',
    'figma-import': 'packages/figma-import/**',
    'capture-browser': 'packages/capture-browser/**',
    sdk: 'packages/sdk/**',
    'agent-plugins': 'packages/agent-plugins/**',
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
      web:               ['contract', 'ui', 'canvas-document', 'artwork-paint', 'capture-browser', 'figma-import', 'api'],
      extension:         ['contract', 'ui', 'capture-browser'],
      storybook:         ['ui'],
      cli:               ['contract', 'canvas-document', 'sdk', 'api'],
      desktop:           ['contract'],
      api:               ['contract', 'artwork-paint', 'canvas-format'],
      docs:              ['cli'],
      contract:          ['artwork-paint', 'canvas-format', 'ui'],
      'canvas-document': ['contract', 'ui'],
      'figma-import':    ['canvas-document', 'contract'],
      sdk:               ['capture-browser', 'contract'],
      'capture-browser': ['contract'],
      'agent-plugins':   ['cli'],
      ui:                [],
      'artwork-paint':   [],
      'canvas-format':   [],
    }, {
      because: 'Apps share code only through packages, and a package never reaches into an app.',
    }),

    rule('web-uses-api-types-only', {
      files: 'web', mayNotImport: 'api', imports: 'runtime',
      because: 'The web app types its requests from the API router. No API code ships to the browser.',
      examples: { rejected: ['apps/web/src/a.ts imports apps/api/src/app.ts'] },
    }),

    rule('core-not-to-driver', {
      files: ['domain', 'application'], mayNotImport: 'driver',
      because: 'The domain and application layers never touch an I/O driver. A use case that needs one declares a port.',
      examples: { rejected: ['apps/api/src/board/domain/a.entity.ts imports pg'] },
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
