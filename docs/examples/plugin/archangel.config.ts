import { defineConfig } from '@enhance-eng/archangel';
import team from './plugins/team.ts';

export default defineConfig({
  parts: { feature: 'src/features/{feature}/**' },
  plugins: { team },
  rules: ({ rule }) => [
    rule('small-files', {
      files: 'feature',
      plugin: 'team/max-imports',
      options: { max: 2 },
      examples: {
        accepted: [{ 'src/features/a/x.ts': "import './y';", 'src/features/a/y.ts': '' }],
        rejected: [
          {
            'src/features/a/x.ts': "import './y';\nimport './z';\nimport './w';",
            'src/features/a/y.ts': '',
            'src/features/a/z.ts': '',
            'src/features/a/w.ts': '',
          },
        ],
      },
    }),
  ],
});
