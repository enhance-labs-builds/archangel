// The Feature-Sliced Design preset: Steiger's 21 rules, fourteen as built-in sentences and seven
// as conditions of the `fsd` plugin. Names and paths are written relative to the preset; archangel
// gives them the preset's name as a prefix and its root as the folder they are in.
import { fsdPlugin } from './fsd-conditions.js';

const LAYERS = ['shared', 'entities', 'features', 'widgets', 'pages', 'app'];
const SLICED = ['entities', 'features', 'widgets', 'pages'];
const SEGMENTS = ['ui', 'api', 'lib', 'model', 'config'];
const RESERVED = [...SEGMENTS, '@x'];
const PURPOSELESS = [
  'component', 'components', 'helper', 'helpers', 'util', 'utils', 'constant', 'constants', 'const',
  'consts', 'type', 'types', 'store', 'stores', 'modal', 'modals', 'service', 'services', 'function',
  'functions', 'class', 'classes', 'enum', 'enums', 'interface', 'interfaces', 'decorator',
  'decorators', 'schema', 'schemas', 'handler', 'handlers', 'fixture', 'fixtures', 'middleware',
  'middlewares', 'validator', 'validators', 'validation', 'validations', 'resolver', 'resolvers',
  'mutation', 'mutations', 'asset', 'assets', 'hook', 'hooks', 'context', 'provider', 'providers',
  'composable', 'composables', 'directive', 'directives', 'action', 'actions', 'reducer',
  'reducers', 'selector', 'selectors', 'effect', 'effects', 'saga', 'sagas', 'thunk', 'thunks',
  'pipe', 'pipes',
];
const OPT_IN = ['no-cross-imports', 'no-higher-level-imports', 'import-locality', 'no-wildcard-exports'];

/** Alternatives in a pattern: `{a,b}`, or the one name itself, since `{a}` would be a capture. */
const either = (names) => (names.length === 1 ? names[0] : `{${names.join(',')}}`);

const higher = (layer) => LAYERS.slice(LAYERS.indexOf(layer) + 1);

const crossImports = SLICED.map((layer) => ({
  files: layer,
  mayNotImport: { all: [layer, { not: `${layer}/*/@x/**` }] },
  instance: 'different',
}));

const higherImports = LAYERS.filter((layer) => higher(layer).length > 0).map((layer) => ({
  files: layer,
  mayNotImport: higher(layer),
}));

const docs = (rule) => `https://feature-sliced.github.io/steiger/rules/${rule}`;

/** An example of `count` files, each made by `file` from a two-digit number. */
const many = (count, file) => Object.fromEntries(Array.from({ length: count }, (_, i) => file(String(i + 1).padStart(2, '0'))));

/** The rules about imports, each with the examples that prove it. */
const IMPORT_RULES = [
  {
    name: 'forbidden-imports',
    reason: 'A module may import only from layers below its own, and a slice not from another slice of its layer, except through @x.',
    link: docs('forbidden-imports'),
    sentences: [...higherImports, ...crossImports],
    examples: {
      rejected: [
        { 'shared/example-lib/index.ts': "import { x } from '../../features/example-login';\n", 'features/example-login/index.ts': 'export const x = 1;\n' },
        { 'features/example-a/model/a.ts': "import { b } from '../../example-b';\n", 'features/example-b/index.ts': 'export const b = 1;\n' },
      ],
      accepted: [
        { 'features/example-login/model/a.ts': "import { y } from '../../../entities/example-user';\n", 'entities/example-user/index.ts': 'export const y = 1;\n' },
        { 'entities/example-a/model/a.ts': "import { b } from '../../example-b/@x/example-a';\n", 'entities/example-b/@x/example-a.ts': 'export const b = 1;\n' },
      ],
    },
  },
  {
    name: 'no-higher-level-imports',
    reason: 'A module may import only from layers below its own.',
    link: docs('no-higher-level-imports'),
    sentences: higherImports,
    examples: {
      rejected: [{ 'shared/example-lib/index.ts': "import { x } from '../../features/example-login';\n", 'features/example-login/index.ts': 'export const x = 1;\n' }],
      accepted: [{ 'features/example-login/model/a.ts': "import { y } from '../../../entities/example-user';\n", 'entities/example-user/index.ts': 'export const y = 1;\n' }],
    },
  },
  {
    name: 'no-cross-imports',
    reason: 'A slice may not import from another slice of its layer, except through its @x public API.',
    link: docs('no-cross-imports'),
    sentences: crossImports,
    examples: {
      rejected: [{ 'features/example-a/model/a.ts': "import { b } from '../../example-b';\n", 'features/example-b/index.ts': 'export const b = 1;\n' }],
      accepted: [{ 'entities/example-a/model/a.ts': "import { b } from '../../example-b/@x/example-a';\n", 'entities/example-b/@x/example-a.ts': 'export const b = 1;\n' }],
    },
  },
  {
    name: 'no-public-api-sidestep',
    reason: "Code outside a slice or segment imports it through its public API, the index file, not its inner files.",
    link: docs('no-public-api-sidestep'),
    sentences: [
      ...SLICED.map((layer) => ({ kind: 'entry', element: layer, files: ['index.*', '@x/**'] })),
      {
        files: { all: ['shared', { not: ['shared/*', 'shared/*/index.*', 'shared/{ui,lib}/*', 'shared/{ui,lib}/*/index.*'] }] },
        mayOnlyBeImportedBy: 'shared',
      },
      { files: { all: ['app', { not: ['app/*', 'app/*/index.*'] }] }, mayOnlyBeImportedBy: 'app' },
    ],
    examples: {
      rejected: [
        { 'pages/example-home/ui/a.ts': "import { x } from '../../../features/example-login/model/x';\n", 'features/example-login/model/x.ts': 'export const x = 1;\n' },
        { 'features/example-login/model/a.ts': "import { x } from '../../../shared/example-api/client';\n", 'shared/example-api/client.ts': 'export const x = 1;\n', 'shared/example-api/index.ts': "export { x } from './client';\n" },
      ],
      accepted: [
        { 'pages/example-home/ui/a.ts': "import { x } from '../../../features/example-login';\n", 'features/example-login/index.ts': 'export const x = 1;\n' },
      ],
    },
  },
];

/** The rules about public APIs and where files and folders are, each with its examples. */
const STRUCTURE_RULES = [
  {
    name: 'public-api',
    reason: 'Each slice, each segment of Shared, and each folder in shared/ui and shared/lib has an index file, its public API.',
    link: docs('public-api'),
    folders: [`${either(SLICED)}/*`, 'shared/*', '!shared/{ui,lib}', 'shared/{ui,lib}/*'],
    mustContain: 'index.*',
    examples: {
      rejected: [{ 'features/example-login/model/x.ts': 'export const x = 1;\n' }],
      accepted: [{ 'features/example-login/index.ts': "export { x } from './model/x';\n", 'features/example-login/model/x.ts': 'export const x = 1;\n' }],
    },
  },
  {
    name: 'no-layer-public-api',
    reason: 'A layer other than App has no index file; its slices and segments have them instead.',
    link: docs('no-layer-public-api'),
    files: `${either(LAYERS.filter((layer) => layer !== 'app'))}/index.*`,
    mustNotExist: true,
    examples: { rejected: [{ 'entities/index.ts': 'export {};\n' }] },
  },
  {
    name: 'no-reserved-folder-names',
    reason: 'A folder inside a segment may not have a name that a segment or @x has, which would be confusing.',
    link: docs('no-reserved-folder-names'),
    folders: [`{shared,app}/*/**/${either(RESERVED)}`, `${either(SLICED)}/*/*/**/${either(RESERVED)}`],
    mustNotExist: true,
    examples: { rejected: [{ 'shared/example-kit/ui/button.ts': 'export {};\n' }] },
  },
  {
    name: 'no-segmentless-slices',
    reason: 'Each slice divides its code into segments, such as ui, api, lib, model and config.',
    link: docs('no-segmentless-slices'),
    folders: `${either(SLICED)}/*`,
    mustContain: [`${either(SEGMENTS)}.*`, `${either(SEGMENTS)}/**`],
    examples: {
      rejected: [{ 'features/example-login/index.ts': 'export {};\n', 'features/example-login/login.ts': 'export {};\n' }],
      accepted: [{ 'features/example-login/index.ts': 'export {};\n', 'features/example-login/model/login.ts': 'export {};\n' }],
    },
  },
  {
    name: 'no-segments-on-sliced-layers',
    reason: 'A segment belongs inside a slice, not directly in a sliced layer.',
    link: docs('no-segments-on-sliced-layers'),
    folders: `${either(SLICED)}/${either(SEGMENTS)}`,
    mustNotExist: true,
    examples: { rejected: [{ 'features/ui/button.ts': 'export {};\n' }] },
  },
  {
    name: 'no-ui-in-app',
    reason: 'The App layer has no ui segment; UI belongs in the layers below it.',
    link: docs('no-ui-in-app'),
    sentences: [
      { folders: 'app/ui', mustNotExist: true },
      { files: 'app/ui.*', mustNotExist: true },
    ],
    examples: { rejected: [{ 'app/ui/root.tsx': 'export {};\n' }, { 'app/ui.tsx': 'export {};\n' }] },
  },
  {
    name: 'segments-by-purpose',
    reason: "A segment's name says what its code is for, not what it is, so not utils, components or hooks.",
    link: docs('segments-by-purpose'),
    sentences: [
      { folders: [`{shared,app}/${either(PURPOSELESS)}`, `${either(SLICED)}/*/${either(PURPOSELESS)}`], mustNotExist: true },
      { files: [`{shared,app}/${either(PURPOSELESS)}.*`, `${either(SLICED)}/*/${either(PURPOSELESS)}.*`], mustNotExist: true },
    ],
    examples: { rejected: [{ 'features/example-login/utils/format.ts': 'export {};\n' }, { 'shared/helpers.ts': 'export {};\n' }] },
  },
  {
    name: 'no-processes',
    reason: 'The Processes layer is deprecated; move its code to Features and App.',
    link: docs('no-processes'),
    folders: 'processes',
    mustNotExist: true,
    examples: { rejected: [{ 'processes/example-checkout/index.ts': 'export {};\n' }] },
  },
];

/** Steiger's opt-in rules about how imports and exports are written, each with its examples. */
const WRITING_RULES = [
  {
    name: 'import-locality',
    reason: 'An import inside a slice is relative, and an import from another slice or layer goes through an alias.',
    link: docs('import-locality'),
    sentences: [
      ...LAYERS.map((layer) => ({ files: layer, mayNotImport: LAYERS.filter((l) => l !== layer), specifier: 'relative' })),
      ...SLICED.map((layer) => ({ files: layer, mayNotImport: layer, instance: 'different', specifier: 'relative' })),
      ...SLICED.map((layer) => ({ files: layer, mayNotImport: layer, instance: 'same', specifier: 'alias' })),
      ...['shared', 'app'].map((layer) => ({ files: layer, mayNotImport: layer, specifier: 'alias' })),
    ],
    examples: {
      rejected: [{ 'features/example-a/model/a.ts': "import { b } from '../../../entities/example-b';\n", 'entities/example-b/index.ts': 'export const b = 1;\n' }],
      accepted: [{ 'features/example-a/model/a.ts': "import { c } from '../lib/c';\n", 'features/example-a/lib/c.ts': 'export const c = 1;\n' }],
    },
  },
  {
    name: 'no-wildcard-exports',
    reason: 'An index file names what it exports, so the public API is explicit.',
    link: docs('no-wildcard-exports'),
    files: `${either(LAYERS)}/**/index.*`,
    mayNotExport: 'star',
    examples: {
      rejected: [{ 'features/example-a/index.ts': "export * from './model/a';\n", 'features/example-a/model/a.ts': 'export const a = 1;\n' }],
      accepted: [{ 'features/example-a/index.ts': "export { a } from './model/a';\n", 'features/example-a/model/a.ts': 'export const a = 1;\n' }],
    },
  },
];

/** The rules, before `strict` chooses which are on. */
const rules = (root) => [...IMPORT_RULES, ...STRUCTURE_RULES, ...WRITING_RULES, ...pluginRules(root)];

const pluginRule = (root, name, reason, examples) => ({
  name,
  reason,
  link: docs(name),
  files: '**',
  plugin: `fsd/${name}`,
  options: { root },
  examples,
});

const pluginRules = (root) => [
  pluginRule(root, 'typo-in-layer-name', 'A folder in the root whose name is close to a layer name is probably that layer, misspelt.', {
    rejected: [{ 'featurez/example.ts': 'export {};\n' }],
    accepted: [{ 'features/example-login/index.ts': 'export {};\n' }],
  }),
  pluginRule(root, 'ambiguous-slice-names', 'A slice named like a segment of Shared could be mistaken for it.', {
    rejected: [{ 'shared/example-kit/index.ts': 'export {};\n', 'features/example-kit/model/a.ts': 'export {};\n' }],
    accepted: [{ 'features/example-login/model/a.ts': 'export {};\n' }],
  }),
  pluginRule(root, 'inconsistent-naming', 'The slices of Entities are all plural or all singular.', {
    rejected: [{ 'entities/example-group/orders/model/a.ts': 'export {};\n', 'entities/example-group/order-line/model/a.ts': 'export {};\n' }],
    accepted: [{ 'entities/example-group/apples/model/a.ts': 'export {};\n', 'entities/example-group/pears/model/a.ts': 'export {};\n' }],
  }),
  pluginRule(root, 'insignificant-slice', 'A slice that at most one other slice uses belongs in that slice.', {
    rejected: [{ 'entities/example-lonely/index.ts': 'export {};\n', 'entities/example-lonely/model/a.ts': 'export {};\n' }],
    accepted: [
      {
        'entities/example-used/index.ts': 'export const used = 1;\n',
        'entities/example-used/model/a.ts': 'export {};\n',
        'pages/example-one/ui/a.ts': "import { used } from '../../../entities/example-used';\nexport const a = used;\n",
        'pages/example-two/ui/b.ts': "import { used } from '../../../entities/example-used';\nexport const b = used;\n",
      },
    ],
  }),
  pluginRule(root, 'repetitive-naming', 'Slices that all repeat one word should be a slice group named for it.', {
    rejected: [{ 'features/example-group/item-save/model/a.ts': 'export {};\n', 'features/example-group/item-load/model/a.ts': 'export {};\n', 'features/example-group/item-drop/model/a.ts': 'export {};\n' }],
    accepted: [{ 'features/example-group/save/model/a.ts': 'export {};\n', 'features/example-group/load/model/a.ts': 'export {};\n', 'features/example-group/drop/model/a.ts': 'export {};\n' }],
  }),
  pluginRule(root, 'excessive-slicing', 'More than 20 slices in a layer or group are hard to find your way in.', {
    rejected: [many(21, (i) => [`widgets/example-group/w${i}/ui/a.ts`, 'export {};\n'])],
    accepted: [{ 'widgets/example-group/header/ui/a.ts': 'export {};\n', 'widgets/example-group/footer/ui/a.ts': 'export {};\n' }],
  }),
  pluginRule(root, 'shared-lib-grouping', 'More than 15 modules in shared/lib are hard to find your way in.', {
    rejected: [many(16, (i) => [`shared/lib/example-${i}.ts`, 'export {};\n'])],
    accepted: [{ 'shared/lib/example-dates/index.ts': 'export {};\n' }],
  }),
];

/**
 * The Feature-Sliced Design preset for the code under `root`. It turns on the rules Steiger
 * recommends, and with `strict` its four opt-in rules too. A second use in one repository takes
 * its own `name`.
 */
export const fsd = ({ root, strict = false, name = 'fsd' }) => ({
  name,
  package: '@enhance-eng/archangel',
  root,
  elements: {
    shared: 'shared/**',
    entities: 'entities/{slice}/**',
    features: 'features/{slice}/**',
    widgets: 'widgets/{slice}/**',
    pages: 'pages/{slice}/**',
    app: 'app/**',
    processes: 'processes/**',
  },
  plugins: { fsd: fsdPlugin },
  rules: rules(root).filter((rule) => strict || !OPT_IN.includes(rule.name)),
});
