import { defineConfig } from '@enhance-eng/archangel';

export default defineConfig({
  rules: ({ rule }) => [
    rule('folder-names', {
      folders: 'src/**',
      mustBeNamed: 'kebab-case',
      because: 'One style for folders, so an import path is easy to guess.',
      examples: { rejected: ['src/features/UserProfile/index.ts'] },
    }),

    rule('component-names', {
      files: 'src/components/**/*.tsx',
      mustBeNamed: 'PascalCase',
      because: 'A component file has the name of the component.',
      examples: { rejected: ['src/components/user-avatar.tsx'] },
    }),

    rule('component-exports', {
      files: 'src/components/**/*.tsx',
      mustExport: '{FileName}',
      because: 'Code imports a component by the name of its file.',
      examples: {
        rejected: [{ 'src/components/Avatar.tsx': 'export function Picture() {}' }],
      },
    }),

    rule('ui-file-names', {
      files: 'src/features/*/ui/**/*.tsx',
      mustBeNamed: ['*.view', '*.container'],
      because: 'A view only shows data. A container gets the data.',
      examples: { rejected: ['src/features/cart/ui/cart-list.tsx'] },
    }),

    rule('feature-entry', {
      folders: 'src/features/*',
      mustContain: 'index.ts',
      because: 'Other code imports a feature through its index file.',
      examples: { rejected: ['src/features/search/ui/search.view.tsx'] },
    }),
  ],
});
