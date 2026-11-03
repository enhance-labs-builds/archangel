import { defineConfig, fsd } from '@enhance-eng/archangel';

export default defineConfig({
  extends: [
    fsd({ root: 'apps/web/src' }),
    fsd({ root: 'apps/admin/src', name: 'fsd-admin', strict: true }),
  ],
  off: ['fsd/insignificant-slice'],
  rules: ({ rule }) => [
    rule('fsd/public-api', {
      folders: 'apps/web/src/features/*',
      mustContain: ['index.ts', 'index.tsx'],
      examples: { rejected: ['apps/web/src/features/login/form.ts'] },
    }),
  ],
});
