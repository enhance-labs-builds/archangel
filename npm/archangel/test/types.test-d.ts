// The config types report a rule's mistakes as the config is written.
import { expectError, expectType } from 'tsd';
import { type Config, defineCondition, defineConfig, definePlugin, fsd, nextjs } from '../index.js';

// "a rule about folders takes only the conditions about folders, and no import option"
expectError(
  defineConfig({
    elements: { src: 'src/**' },
    rules: ({ rule }) => [rule({ folders: 'src/**', mustExport: 'x' })],
  }),
);
expectError(
  defineConfig({
    elements: { src: 'src/**' },
    rules: ({ rule }) => [rule({ files: 'src/**', mustBeNamed: 'kebab-case', imports: 'all' })],
  }),
);

// "rule naming an undeclared element is a type error"
expectError(
  defineConfig({
    elements: { entity: 'src/entities/{entity}/**', react: { packages: ['react'] } },
    rules: ({ rule }) => [rule({ files: 'entity', mayNotImport: 'reactt' })],
  }),
);
expectError(
  defineConfig({
    elements: { entity: 'src/entities/{entity}/**', react: { packages: ['react'] } },
    rules: [{ files: 'entity', mayNotImport: 'reactt' }],
  }),
);

// "isolate on an element without a capture is a type error"
expectError(
  defineConfig({
    elements: { shared: 'src/shared/**' },
    rules: ({ isolate }) => [isolate('shared')],
  }),
);
expectError(
  defineConfig({
    elements: { both: 'src/{a,b}/**' },
    rules: ({ isolate }) => [isolate('both')],
  }),
);

// "a sentence naming an undeclared selector is a type error"
expectError(
  defineConfig({
    elements: { src: 'src/**' },
    selectors: { view: '**/*.view.tsx', test: '**/*.test.ts' },
    rules: ({ rule }) => [rule({ files: 'vieww', mayNotImport: 'test' })],
  }),
);

// "a sentence with two conditions is a type error"
expectError(
  defineConfig({
    elements: { src: 'src/**' },
    selectors: { view: '**/*.view.tsx', test: '**/*.test.ts' },
    rules: ({ rule }) => [rule({ files: 'view', mayNotImport: 'test', mustExport: '{FileName}' })],
  }),
);
expectError(
  defineConfig({
    elements: { src: 'src/**' },
    selectors: { view: '**/*.view.tsx', test: '**/*.test.ts' },
    rules: ({ rule }) => [rule({ files: 'view', mayNotImport: 'test', mayOnlyImport: 'src/**' })],
  }),
);

// A config that names only declared elements, and isolates an element with a capture, is fine.
const good = defineConfig({
  elements: {
    feature: 'src/features/{feature}/**',
    shared: ['src/shared/**', 'src/lib/**'],
    react: { packages: ['react', 'react-dom'] },
  },
  ignore: ['src/generated/'],
  allowUnresolved: { 'virtual:*': 'Vite provides these at build time' },
  selectors: {
    view: '**/*.view.tsx',
    test: ['**/*.test.ts', '**/*.test.tsx'],
    client: { directive: 'use client' },
    production: { all: ['src/**', { not: 'test' }] },
  },
  rules: ({ layers, isolate, entry, rule }) => [
    layers(['feature', 'shared'], { reason: 'Shared code must not know about features' }),
    isolate('feature'),
    entry('feature', 'index.ts'),
    rule({ files: 'shared', mayNotImport: 'react', imports: 'runtime' }),
    rule({ name: 'thin-views', files: 'view', mayOnlyImport: ['shared', 'react', '**/*.css'] }),
    rule({ files: { not: 'test' }, mayNotImport: 'test' }),
    rule({ files: 'feature', mayNotImport: 'feature', instance: 'different', specifier: 'relative' }),
    rule({ files: 'src/**', mayNotImport: { packages: ['node:*'] }, names: ['env'] }),
    rule({ files: 'src/**', mayNotImport: { packages: ['*'] }, dependency: 'undeclared' }),
    rule({
      files: 'client',
      mayNotImport: { packages: ['server-only'] },
      transitively: true,
      stopAt: { directive: 'use server' },
    }),
    rule({ files: { all: ['src/**', { not: { element: '*' } }] }, mustNotExist: true }),
    rule({ files: 'src/**', noCycles: true }),
    rule({ files: 'src/**/*.tsx', mustBeNamed: 'PascalCase' }),
    rule({ files: 'test', mustBeNamed: '{folder-name}.test' }),
    rule({ folders: 'src/**', mustBeNamed: 'kebab-case' }),
    rule({ files: 'feature', mustContain: 'index.ts' }),
    rule({ folders: ['src/features/*'], mustContain: 'README.md' }),
    rule({ files: 'view', mustExport: '{FileName}' }),
    rule({ files: 'src/**', mayNotExport: 'default' }),
    rule({ files: 'src/**', mayNotExport: 'internal*' }),
    rule({ files: '**/index.ts', mustOnlyReExport: true }),
  ],
});
expectType<Config<
  {
    readonly feature: 'src/features/{feature}/**';
    readonly shared: readonly ['src/shared/**', 'src/lib/**'];
    readonly react: { readonly packages: readonly ['react', 'react-dom'] };
  },
  {
    readonly view: '**/*.view.tsx';
    readonly test: readonly ['**/*.test.ts', '**/*.test.tsx'];
    readonly client: { readonly directive: 'use client' };
    readonly production: { readonly all: readonly ['src/**', { readonly not: 'test' }] };
  }
>>(good);

// Plugins: a rule names a loaded plugin's condition, and a condition sees the facts it reads.
const oneStore = defineCondition({
  name: 'one-store',
  description: 'Each entity has at most `max` stores.',
  optionsSchema: { type: 'object', properties: { max: { type: 'number' } } },
  reads: ['imports'],
  check: (view, options: { max: number }) =>
    view.files
      .filter((file) => file.imports.length > options.max)
      .map((file) => ({ file: file.path, message: `${file.imports.length} imports` })),
});
// A condition's JSON Schema is `optionsSchema`; `options` are what a rule passes to `check`.
expectError(
  defineCondition({
    name: 'old-name',
    description: 'Uses the schema field under the name of the values.',
    options: { type: 'object' },
    reads: [],
    check: () => [],
  }),
);
const team = definePlugin({ name: 'team', schemaVersion: 1, conditions: [oneStore] });
defineConfig({
  elements: { entity: 'src/entities/{entity}/**' },
  plugins: { team },
  rules: ({ rule }) => [rule({ files: 'entity', plugin: 'team/one-store', options: { max: 1 } })],
});
expectError(
  defineConfig({
    elements: { entity: 'src/entities/{entity}/**' },
    plugins: { team },
    rules: [{ files: 'entity', plugin: 'team/one-stores' }],
  }),
);
expectError(
  defineCondition({
    name: 'exports-only',
    description: 'A condition that reads exports sees no imports.',
    reads: ['exports'],
    check: (view) => view.files.map((file) => ({ file: file.path, message: `${file.imports.length}` })),
  }),
);

// Presets.
defineConfig({ extends: [fsd({ root: 'src' })], off: ['fsd/insignificant-slice'], rules: [] });
defineConfig({
  elements: { app: 'src/app/**' },
  extends: [fsd({ root: 'apps/web/src', strict: true }), fsd({ root: 'apps/admin/src', name: 'fsd-admin' }), nextjs()],
  rules: [{ name: 'fsd/public-api', folders: ['src/*', '!src/app'], mustContain: ['index.ts', 'index.tsx'] }],
});
expectError(defineConfig({ extends: [fsd({ strict: true })], rules: [] }));
defineConfig({
  elements: { app: 'src/app/**', feature: 'src/features/{feature}/**' },
  rules: [
    {
      name: 'no-ui-in-app',
      sentences: [
        { folders: 'src/app/ui', mustNotExist: true },
        { files: 'src/app/ui.*', mustNotExist: true },
        { kind: 'isolate', element: 'feature' },
      ],
    },
  ],
});
expectError(
  defineConfig({
    elements: { app: 'src/app/**' },
    rules: [{ sentences: [{ folders: 'src/app/ui', mustNotExist: true }] }],
  }),
);

// "a config's own rule name cannot contain a slash, which separates a preset's name from its rule's"
expectError(
  defineConfig({
    elements: { src: 'src/**' },
    rules: [{ name: 'team/no-legacy', files: 'src/**', mayNotImport: 'src/legacy/**' }],
  }),
);
expectError(
  defineConfig({
    elements: { src: 'src/**' },
    rules: ({ rule }) => [rule({ name: 'team/no-legacy', files: 'src/**', mustNotExist: true })],
  }),
);
expectError(
  defineConfig({
    elements: { app: 'src/app/**', shared: 'src/shared/**' },
    rules: ({ layers }) => [layers(['app', 'shared'], { name: 'team/layers' })],
  }),
);
expectError(
  defineConfig({
    elements: { feature: 'src/features/{feature}/**' },
    rules: ({ isolate }) => [isolate('feature', { name: 'team/isolate' })],
  }),
);
expectError(
  defineConfig({
    elements: { entity: 'src/entities/{entity}/**' },
    rules: ({ entry }) => [entry('entity', 'index.ts', { name: 'team/entry' })],
  }),
);
expectError(
  defineConfig({
    elements: { app: 'src/app/**' },
    rules: [{ name: 'team/no-ui', sentences: [{ folders: 'src/app/ui', mustNotExist: true }] }],
  }),
);
expectError(
  defineConfig({
    elements: { app: 'src/app/**' },
    rules: [{ name: 'team/no-ui', folders: 'src/app/ui', mustNotExist: true }],
  }),
);
expectError(
  defineConfig({
    elements: { app: 'src/app/**' },
    plugins: { team },
    rules: [{ name: 'team/one-store', files: 'app', plugin: 'team/one-store', options: { max: 1 } }],
  }),
);
// The prefix of a preset the config extends names the rule it replaces; any other prefix does not.
expectError(
  defineConfig({
    extends: [fsd({ root: 'src' }), nextjs()],
    rules: [{ name: 'team/public-api', folders: 'src/*', mustContain: 'index.ts' }],
  }),
);
expectError(
  defineConfig({
    extends: [fsd({ root: 'src', name: 'fsd-admin' })],
    rules: [{ name: 'fsd/public-api', folders: 'src/*', mustContain: 'index.ts' }],
  }),
);
defineConfig({
  extends: [fsd({ root: 'src', name: 'fsd-admin' }), nextjs()],
  rules: ({ rule }) => [
    rule({ name: 'fsd-admin/public-api', folders: 'src/*', mustContain: 'index.ts' }),
    rule({ name: 'nextjs/client-stays-client', files: 'src/**', mayNotImport: 'src/server/**' }),
    rule({ name: 'no-legacy', files: 'src/**', mayNotImport: 'src/legacy/**' }),
  ],
});

// "a table cell that names no row, no declared name and no pattern is a type error"
expectError(
  defineConfig({
    parts: { api: 'apps/api/**' },
    rules: ({ mayImport }) => [
      mayImport('domain', { entity: ['value-objects'], 'value-object': [] }, { kinds: '**/*.{kind}.ts' }),
    ],
  }),
);
defineConfig({
  parts: { api: 'apps/api/**' },
  externals: { driver: ['pg'] },
  rules: ({ mayImport }) => [
    mayImport(
      'domain',
      { entity: ['value-object', 'api', 'driver', 'apps/api/src/shared/result.ts'], 'value-object': [] },
      { kinds: '**/*.{kind}.ts' },
    ),
  ],
});

// "a row of a table without kinds must be a declared part or group"
expectError(
  defineConfig({
    parts: { 'canvas-document': 'packages/canvas-document/**' },
    rules: ({ mayImport }) => [mayImport('workspace', { 'canvas-documents': [] })],
  }),
);
expectError(
  defineConfig({
    parts: { app: 'src/**' },
    externals: { driver: ['pg'] },
    rules: ({ mayImport }) => [mayImport('workspace', { driver: [] })],
  }),
);
defineConfig({
  parts: { 'canvas-document': 'packages/canvas-document/**' },
  groups: { tests: '**/*.test.ts' },
  rules: ({ mayImport }) => [mayImport('workspace', { 'canvas-document': [], tests: ['canvas-document'] })],
});

// "a table or row name with a colon or slash is a type error"
expectError(
  defineConfig({
    parts: { app: 'src/**' },
    rules: ({ mayImport }) => [mayImport('api:domain', { app: [] })],
  }),
);
expectError(
  defineConfig({
    parts: { app: 'src/**' },
    rules: ({ mayImport }) => [mayImport('domain', { 'aggregate/root': [] }, { kinds: '**/*.{kind}.ts' })],
  }),
);
defineConfig({
  parts: { app: 'src/**' },
  rules: ({ mayImport }) => [mayImport('api.domain', { 'my views': [] }, { kinds: '**/*.{kind}.ts' })],
});

// The other options of a table are typed too.
expectError(
  defineConfig({
    parts: { app: 'src/**' },
    rules: ({ mayImport }) => [
      mayImport('domain', { entity: [] }, { kinds: '**/*.{kind}.ts', mayNotImportItself: ['entities'] }),
    ],
  }),
);
expectError(
  defineConfig({
    parts: { app: 'src/**' },
    rules: ({ mayImport }) => [mayImport('domain', { app: [] }, { files: 'apps' })],
  }),
);

// "within is typed as a rule side and only on mayOnlyImport and mayOnlyBeImportedBy"
expectError(
  defineConfig({
    parts: { entity: 'src/entity/**' },
    rules: ({ rule }) => [rule('names', { files: 'entity', mustBeNamed: 'kebab-case', within: 'src/**' })],
  }),
);
expectError(
  defineConfig({
    parts: { entity: 'src/entity/**', shared: 'src/shared/**' },
    rules: ({ rule }) => [rule('names', { files: 'entity', mayNotImport: 'shared', within: 'src/**' })],
  }),
);
expectError(
  defineConfig({
    parts: { entity: 'src/entity/**', shared: 'src/shared/**' },
    rules: ({ rule }) => [rule('names', { files: 'entity', mayOnlyImport: 'shared', within: 'domains' })],
  }),
);
defineConfig({
  parts: { entity: 'src/entity/**', shared: 'src/shared/**' },
  groups: { domain: 'src/domain/**' },
  rules: ({ rule }) => [
    rule('only', { files: 'entity', mayOnlyImport: 'shared', within: 'src/**' }),
    rule('by', { files: 'entity', mayOnlyBeImportedBy: 'shared', within: 'domain' }),
  ],
});

// "a config with two spellings of one key is a type error"
expectError(
  defineConfig({
    parts: { app: 'src/**' },
    elements: { app: 'src/**' },
    rules: [],
  }),
);
expectError(
  defineConfig({
    parts: { app: 'src/**' },
    groups: { test: '**/*.test.ts' },
    selectors: { test: '**/*.test.ts' },
    rules: [],
  }),
);
expectError(
  defineConfig({
    parts: { app: 'src/**' },
    rules: ({ rule }) => [rule({ files: 'app', noCycles: true, because: 'a', reason: 'a' })],
  }),
);
expectError(
  defineConfig({
    parts: { app: 'src/**' },
    rules: ({ rule }) => [rule('a', { name: 'b', files: 'app', noCycles: true })],
  }),
);

// A part holds files, so `parts` has no packages; `externals` holds them.
expectError(
  defineConfig({
    parts: { react: { packages: ['react'] } },
    rules: [],
  }),
);

// "an example item is a string or a file set"
defineConfig({
  parts: { app: 'src/**' },
  rules: ({ rule }) => [
    rule('a', {
      files: 'app',
      noCycles: true,
      examples: {
        accepted: ['src/a.ts imports src/b.ts'],
        rejected: ['src/a.ts imports src/b.ts imports src/a.ts', { 'src/a.ts': "import './a';" }],
      },
    }),
  ],
});
expectError(
  defineConfig({
    parts: { app: 'src/**' },
    rules: ({ rule }) => [rule('a', { files: 'app', noCycles: true, examples: { rejected: [1] } })],
  }),
);

// "a bare string is not an example list"
expectError(
  defineConfig({
    parts: { app: 'src/**' },
    rules: ({ rule }) => [rule('a', { files: 'app', noCycles: true, examples: { rejected: 'src/a.ts' } })],
  }),
);

// "the configs of the checked fixtures type-check with literal names"
import '../../../docs/examples/small-app/archangel.config.js';
import '../../../docs/examples/enhance3-slice/archangel.config.js';
