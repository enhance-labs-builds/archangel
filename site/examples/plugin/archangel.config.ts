import { defineConfig } from '@enhance-eng/archangel';
import team from './plugins/team.ts';

export default defineConfig({
  parts: {
    feature: 'src/features/{feature}/**',
    shared: 'src/shared/**',
  },
  plugins: { team },
  rules: ({ rule }) => [
    rule('small-files', {
      files: 'feature',
      plugin: 'team/max-imports',
      options: { max: 2 },
      because: 'A file with many imports does too many things.',
      examples: {
        accepted: [{ 'src/features/a/x.ts': "import './y';", 'src/features/a/y.ts': '' }],
        rejected: [{
          'src/features/a/x.ts': "import './y';\nimport './z';\nimport './w';",
          'src/features/a/y.ts': '',
          'src/features/a/z.ts': '',
          'src/features/a/w.ts': '',
        }],
      },
    }),

    rule('no-orphans', {
      files: 'shared',
      plugin: 'team/no-orphans',
      because: 'Shared code that nothing imports is dead code.',
      examples: {
        rejected: [{ 'src/shared/unused.ts': 'export const unused = 1;' }],
      },
    }),
  ],
});
