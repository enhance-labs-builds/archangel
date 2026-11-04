import { defineConfig } from '@enhance-eng/archangel';

export default defineConfig({
  parts: {
    feature: 'src/features/{feature}/**',   // each folder in src/features is one feature
    shared: 'src/shared/**',
  },
  rules: ({ mayImport }) => [
    mayImport('app', {
      feature: ['shared'],
      shared: [],
    }, {
      because: 'Features share code only through src/shared, and shared code never depends on a feature.',
    }),

    mayImport('feature-code', {
      views: ['models', 'shared'],
      models: ['api', 'shared'],
      api: ['shared'],
    }, {
      kinds: 'src/features/*/{kind}/**',     // src/features/cart/views/cart-page.tsx is in views
      because: 'A view shows a model. Only the api folder talks to the server.',
    }),
  ],
});
