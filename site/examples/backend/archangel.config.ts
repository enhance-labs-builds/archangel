import { defineConfig } from '@enhance-eng/archangel';

export default defineConfig({
  parts: {
    routes: 'src/routes/**',
    services: 'src/services/**',
    db: 'src/db/**',
    domain: 'src/domain/**',
  },
  externals: {
    driver: ['pg'],
  },
  rules: ({ mayImport, rule }) => [
    mayImport('backend', {
      routes: ['services'],
      services: ['domain', 'db'],
      db: ['domain', 'driver'],
      domain: [],
    }, {
      because: 'A route calls a service. Only db talks to the database. The domain depends on nothing.',
    }),

    mayImport('model', {
      entity: ['value-object'],
      event: ['entity', 'value-object'],
      'value-object': [],
    }, {
      files: 'domain',
      kinds: 'src/domain/*.{kind}.ts',
      because: 'An entity does not know its events. A service publishes them.',
    }),

    rule('no-cycles', {
      files: 'src/**',
      noCycles: true,
      because: 'Files that import each other cannot change one at a time.',
      examples: {
        rejected: ['src/services/a.ts imports src/services/b.ts imports src/services/a.ts'],
      },
    }),
  ],
});
