import checkFile from 'eslint-plugin-check-file';

export default [
  {
    files: ['src/**/*.{ts,tsx}'],
    plugins: { 'check-file': checkFile },
    rules: {
      'check-file/filename-naming-convention': [
        'error',
        { 'src/**/*.{ts,tsx}': 'KEBAB_CASE' },
        { ignoreMiddleExtensions: true },
      ],
      'check-file/folder-naming-convention': ['error', { 'src/**/': 'KEBAB_CASE' }],
      'check-file/filename-blocklist': ['error', { '**/*.util.ts': '*.helpers.ts' }],
      'check-file/folder-match-with-fex': ['error', { '*.test.ts': '**/__tests__/' }],
    },
  },
];
