import { defineConfig } from '@enhance-eng/archangel';

export default defineConfig({
  parts: {
    module: 'src/{area}/{module}/**',
  },
  rules: ({ isolate }) => [
    isolate('module', {
      because: 'A module changes alone. Modules talk through events, not imports.',
      examples: {
        rejected: ['src/sales/orders/a.ts imports src/sales/quotes/b.ts'],
      },
    }),
  ],
});
