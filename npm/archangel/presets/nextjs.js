// The Next.js preset: code that a client component reaches stays out of server-only code and
// Node's built-in modules.

const page = (text) => `'use client';\n${text}export default function Page() { return null; }\n`;

/**
 * The Next.js preset. A file whose first statement is `'use client'`, and every file it reaches
 * through runtime imports, may not import `server-only` or a Node built-in module; a chain stops
 * at a file that starts with `'use server'`, whose import Next.js turns into a network call.
 */
export const nextjs = ({ name = 'nextjs', root = '' } = {}) => ({
  name,
  package: '@enhance-eng/archangel',
  root,
  selectors: {
    client: { directive: 'use client' },
    server: { directive: 'use server' },
  },
  rules: [
    {
      name: 'client-stays-client',
      reason: 'Code that runs in the browser cannot use server-only code or Node built-in modules.',
      link: 'https://nextjs.org/docs/app/getting-started/server-and-client-components',
      files: 'client',
      mayNotImport: { packages: ['server-only', 'node:*'] },
      transitively: true,
      stopAt: 'server',
      imports: 'runtime',
      examples: {
        rejected: [
          {
            'app/example-page.tsx': page("import { read } from './example-helper';\n"),
            'app/example-helper.ts': "import { readFile } from 'node:fs';\nexport const read = readFile;\n",
          },
        ],
        accepted: [
          {
            'app/example-page.tsx': page("import { save } from './example-actions';\n"),
            'app/example-actions.ts': "'use server';\nimport { readFile } from 'node:fs';\nexport async function save() { return readFile; }\n",
          },
        ],
      },
    },
  ],
});
