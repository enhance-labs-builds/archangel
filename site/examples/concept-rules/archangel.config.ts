import { defineConfig } from '@enhance-eng/archangel';

export default defineConfig({
  parts: {
    app: 'src/app/**',
    feature: 'src/features/{feature}/**',
    shared: 'src/shared/**',
  },
  rules: ({ entry, layers, rule }) => [
    layers(['app', 'feature', 'shared'], {
      because: 'Code depends only on the layers below it.',
      examples: { rejected: ['src/shared/a.ts imports src/app/main.ts'] },
    }),

    entry('feature', 'index.ts', {
      examples: { rejected: ['src/app/main.ts imports src/features/cart/cart.ts'] },
    }),

    rule('no-react-in-shared', {
      files: 'shared',
      mayNotImport: { packages: ['react'] },
      transitively: true,
      because: 'Shared code also runs on the server.',
      examples: { rejected: ['src/shared/a.ts imports src/shared/b.ts imports react'] },
    }),

    rule('no-utils', {
      sentences: [
        { folders: 'src/**/utils', mustNotExist: true },
        { files: 'src/**/utils.ts', mustNotExist: true },
      ],
      because: 'A name says what the code does, not that it is a utility.',
      examples: { rejected: ['src/shared/utils.ts'] },
    }),
  ],
});
