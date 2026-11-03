module.exports = {
  forbidden: [
    {
      name: 'no-circular',
      severity: 'error',
      from: {},
      to: { circular: true },
    },
    {
      name: 'ui-not-to-db',
      severity: 'error',
      comment: 'The UI reads data through the api folder.',
      from: { path: '^src/ui' },
      to: { path: '^src/db' },
    },
    {
      name: 'not-to-dev-dep',
      severity: 'error',
      from: { path: '^src', pathNot: '\\.test\\.ts$' },
      to: { dependencyTypes: ['npm-dev'] },
    },
  ],
};
