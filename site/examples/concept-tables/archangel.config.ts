import { defineConfig } from '@enhance-eng/archangel';

export default defineConfig({
  parts: {
    ui: 'src/ui/**',
    domain: 'src/domain/**',
    db: 'src/db/**',
    log: 'src/log.ts',
  },
  externals: {
    driver: ['pg'],
    view: ['react'],
  },
  rules: ({ mayImport }) => [
    mayImport('app', {
      ui: ['domain', 'view'],
      domain: [],
      db: ['domain', 'driver'],
      log: [],
    }, {
      everyRowMayImport: ['log'],
      because: 'The UI shows the domain. Only db talks to the database.',
    }),
  ],
});
