// The published JSON schema accepts the configs archangel accepts, and refuses what it refuses.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import Ajv from 'ajv';

const read = (path) => JSON.parse(readFileSync(new URL(path, import.meta.url), 'utf8'));
const validate = new Ajv({ allErrors: true, strict: true }).compile(read('../archangel.schema.json'));

test('the configs in this repository are valid', () => {
  for (const path of [
    '../../archangel.config.json',
    '../../../benches/configs/vscode.json',
    '../../../benches/configs/excalidraw.json',
  ]) {
    assert.ok(validate(read(path)), `${path}: ${JSON.stringify(validate.errors)}`);
  }
});

test('a config with every element form, selector form, rule kind, condition and option is valid', () => {
  const config = {
    $schema: './node_modules/@enhance-eng/archangel/archangel.schema.json',
    elements: {
      app: 'src/app/**',
      feature: ['src/features/{feature}/**'],
      db: 'src/db/**',
      react: { packages: ['react', 'react-dom'] },
    },
    selectors: {
      view: '**/*.view.tsx',
      test: ['**/*.test.ts', 'feature'],
      production: { all: ['src/**', { not: 'test' }] },
      either: { any: ['app', 'db'] },
      client: { directive: 'use client' },
      features: { element: 'feature' },
      login: { element: 'feature', capture: { feature: 'login' } },
      placed: { element: '*' },
    },
    ignore: ['generated/'],
    allowUnresolved: { 'virtual:*': 'Vite makes these' },
    rules: [
      { kind: 'layers', order: ['app', 'feature'], reason: 'r', link: 'l', imports: 'runtime' },
      { kind: 'isolate', element: 'feature', examples: { accepted: [{}], rejected: [{ 'a.ts': '' }] } },
      { kind: 'entry', element: 'feature', files: ['index.ts'] },
      { files: 'db', mayOnlyBeImportedBy: ['app'], name: 'db-behind-app' },
      { files: 'feature', mayNotImport: 'react', names: ['useEffect'], imports: 'types' },
      { files: 'feature', mayNotImport: 'feature', instance: 'different', specifier: 'relative' },
      { files: 'src/**', mayNotImport: { packages: ['*'] }, dependency: 'undeclared' },
      {
        files: 'client',
        mayNotImport: [{ packages: ['server-only', 'node:*'] }, { directive: 'use server' }],
        transitively: true,
        stopAt: { directive: 'use server' },
        imports: 'runtime',
      },
      { files: 'view', mayOnlyImport: ['react', '**/*.css', { element: 'feature' }], dependency: 'dev' },
      { files: { all: ['src/**', { not: 'placed' }] }, mustNotExist: true },
      { files: '**', noCycles: true, imports: 'all' },
    ],
  };
  assert.ok(validate(config), JSON.stringify(validate.errors));
});

test('a config with every structure and export condition is valid', () => {
  const config = {
    elements: { feature: 'src/features/{feature}/**' },
    selectors: { view: '**/*.view.tsx', test: '**/*.test.ts' },
    rules: [
      { files: 'src/**/*.tsx', mustBeNamed: 'PascalCase' },
      { files: 'test', mustBeNamed: '{folder-name}.test' },
      { folders: 'src/**', mustBeNamed: 'kebab-case' },
      { files: 'feature', mustContain: 'index.ts' },
      { folders: ['src/features/*'], mustContain: 'README.md' },
      { files: 'view', mustExport: '{FileName}' },
      { files: 'src/**', mayNotExport: 'star' },
      { files: 'src/**', mayNotExport: 'internal*' },
      { files: '**/index.ts', mustOnlyReExport: true },
    ],
  };
  assert.ok(validate(config), JSON.stringify(validate.errors));
});

test('what archangel refuses, the schema refuses', () => {
  const elements = { app: 'src/app/**' };
  for (const config of [
    { elements, rules: [], rulez: [] },
    { elements, plugins: {}, rules: [] },
    { elements, rules: [{ files: 'app', plugin: 'team/one-store' }] },
    { elements, selectorz: {}, rules: [] },
    { elements, rules: [{ kind: 'isolat', element: 'app' }] },
    { elements, rules: [{ kind: 'only', element: 'app', importedBy: ['app'] }] },
    { elements, rules: [{ kind: 'forbid', from: ['app'], to: ['app'] }] },
    { elements, rules: [{ kind: 'noCycles' }] },
    { elements, rules: [{ files: '**', noCycles: true, reasn: 'typo' }] },
    { elements, rules: [{ files: 'app', mayNotImport: 'app', mayOnlyImport: 'app' }] },
    { elements, rules: [{ files: 'app' }] },
    { elements, rules: [{ mayNotImport: 'app' }] },
    { elements, rules: [{ files: '**', noCycles: false }] },
    { elements, rules: [{ files: '**', noCycles: true, names: ['x'] }] },
    { elements, rules: [{ files: 'app', mayOnlyImport: 'app', transitively: true }] },
    { elements, rules: [{ files: 'app', mayNotImport: 'app', stopAt: 'app' }] },
    { elements, rules: [{ files: 'app', mayNotImport: 'app', transitively: true, names: ['x'] }] },
    { elements, rules: [{ files: 'app', mayNotImport: 'app', transitively: true, imports: 'types' }] },
    { elements, rules: [{ files: 'app', mayNotImport: 'app', specifier: 'absolute' }] },
    { elements, rules: [{ files: { all: ['app'], any: ['app'] }, noCycles: true }] },
    { elements, rules: [{ files: { element: '*', capture: { x: 'y' } }, noCycles: true }] },
    { elements, selectors: { react: { packages: ['react'] } }, rules: [] },
    { elements, rules: [{ kind: 'layers' }] },
    { elements, rules: [{ files: '**', noCycles: true, imports: 'some' }] },
    { elements: { react: { packages: [] } }, rules: [] },
    { rules: [] },
    { elements, rules: [{ folders: 'src/**', mustExport: 'x' }] },
    { elements, rules: [{ files: 'app', folders: 'src/**', mustBeNamed: 'kebab-case' }] },
    { elements, rules: [{ files: 'app', mustBeNamed: 'kebab-case', imports: 'all' }] },
    { elements, rules: [{ files: 'app', mustOnlyReExport: false }] },
  ]) {
    assert.equal(validate(config), false, JSON.stringify(config));
  }
});
