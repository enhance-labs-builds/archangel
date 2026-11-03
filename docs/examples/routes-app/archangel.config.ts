import { defineConfig } from '@enhance-eng/archangel';

export default defineConfig({
  parts: {
    routes: 'src/routes/**',
    services: 'src/services/**',
    db: 'src/db/**',
  },
  rules: ({ mayImport, rule }) => [
    mayImport('layers', {
      routes: ['services'],
      services: ['db'],
      db: [],
    }, {
      because: 'A route calls a service. Only a service calls the database code.',
    }),

    rule('no-pg-in-routes', {
      files: 'routes',
      mayNotImport: { packages: ['pg'] },
      because: 'Only the db folder talks to the database.',
      examples: {
        accepted: ['src/routes/users.ts imports src/services/users.ts'],
        rejected: ['src/routes/users.ts imports pg'],
      },
    }),
  ],
});
