// Spec: openspec/changes/documentation-site, "Each config on a page comes from a fixture" and
// "Each rule fragment on a page loads".
import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { join, relative } from 'node:path';
import { test } from 'node:test';
import { pages, repoOf, run, shown } from './support.mjs';

const BUILDERS = ['rule(', 'mayImport(', 'layers(', 'isolate(', 'entry('];

/**
 * Each page that the site writes itself. The pages of diagnostic codes are the release's own text,
 * which `archangel docs` prints, so the site does not check them.
 */
function pageFiles() {
  return readdirSync(pages, { recursive: true })
    .filter((path) => /\.mdx?$/.test(path) && !/^reference[\\/]diagnostics[\\/]/.test(path))
    .map((path) => join(pages, path));
}

/** Each fenced block of a page, with its language and the line it starts on. */
function blocks(text) {
  const found = [];
  let open = null;
  text.split('\n').forEach((line, index) => {
    const fence = line.match(/^\s*```(\S*)/);
    if (fence && open === null) open = { lang: fence[1], line: index + 1, code: '' };
    else if (fence && open !== null) {
      found.push(open);
      open = null;
    } else if (open !== null) open.code += `${line}\n`;
  });
  return found;
}

/** A problem for each whole config that a page writes itself. */
export function configsWritten(page, text) {
  return blocks(text)
    .filter((block) => block.lang === 'ts' && block.code.includes('defineConfig('))
    .map((block) => `${page}:${block.line}: this page writes a whole config; import it from a fixture`);
}

/** Whether code is a rule fragment that can load alone. A placeholder and a plugin rule cannot. */
function isFragment(code) {
  const trimmed = code.trim();
  return (
    BUILDERS.some((builder) => trimmed.startsWith(builder)) &&
    trimmed.includes("'") &&
    !trimmed.includes('...') &&
    !trimmed.includes('plugin:')
  );
}

/** Each rule fragment of a page, in a ts block or in inline code, with its line. */
export function fragments(text) {
  const found = blocks(text)
    .filter((block) => block.lang === 'ts' && isFragment(block.code))
    .map((block) => ({ line: block.line, code: block.code }));
  let fenced = false;
  text.split('\n').forEach((line, index) => {
    if (/^\s*```/.test(line)) fenced = !fenced;
    if (fenced) return;
    line
      .split('`')
      .filter((_, at) => at % 2 === 1)
      .filter(isFragment)
      .forEach((code) => found.push({ line: index + 1, code }));
  });
  return found;
}

/** The names that a fragment on a page may use, each with files so that no set is empty. */
const PARTS = ['app', 'shared', 'view', 'db', 'api', 'entity', 'value-object', 'error', 'use-case', 'routes', 'services', 'domain', 'ui'];

/** A repository whose config holds one fragment, with each name that the pages use. */
function fragmentRepo(code) {
  const files = { 'src/features/login/index.ts': '', 'src/shared/a.ts': '' };
  for (const part of PARTS) files[`src/${part}/a.ts`] = '';
  const parts = PARTS.map((name) => `'${name}': 'src/${name}/**'`).join(', ');
  files['archangel.config.mjs'] = `export default {
  parts: { feature: 'src/features/{feature}/**', ${parts} },
  groups: { test: '**/*.test.ts', client: { directive: 'use client' } },
  externals: { driver: ['pg'] },
  rules: ({ rule, mayImport, layers, isolate, entry }) => [${code}],
};
`;
  files['package.json'] = '{ "name": "fragment", "private": true, "dependencies": { "react": "1.0.0", "react-dom": "1.0.0", "pg": "1.0.0" } }\n';
  return repoOf(files);
}

/** A problem for each fragment of a page that does not load. */
export function fragmentsThatDoNotLoad(page, text) {
  return fragments(text).flatMap(({ line, code }) => {
    const found = run(fragmentRepo(code), ['check']);
    return found.status === 2 ? [`${page}:${line}: this fragment does not load: ${code}\n${shown(found)}`] : [];
  });
}

test('a config that a page writes itself is named', () => {
  const text = '# Rules\n\n```ts\nexport default defineConfig({ parts: {} });\n```\n';
  assert.deepEqual(configsWritten('guides/a.mdx', text), [
    'guides/a.mdx:3: this page writes a whole config; import it from a fixture',
  ]);
});

test('no page writes a whole config', () => {
  const found = pageFiles().flatMap((path) => configsWritten(relative(pages, path), readFileSync(path, 'utf8')));
  assert.deepEqual(found, [], found.join('\n'));
});

test('a fragment that does not load is named', () => {
  const text = "Write `rule('a', { files: 'shared', mayNotImport: 'react' })` in a config.\n";
  const found = fragmentsThatDoNotLoad('reference/a.mdx', text);
  assert.equal(found.length, 1, found.join('\n'));
  assert.match(found[0], /^reference\/a\.mdx:1: this fragment does not load/);
});

test('each rule fragment on a page loads', () => {
  const found = pageFiles().flatMap((path) => fragmentsThatDoNotLoad(relative(pages, path), readFileSync(path, 'utf8')));
  assert.deepEqual(found, [], found.join('\n'));
});

test('the card on the home page quotes the first table and its error', () => {
  const home = readFileSync(join(pages, 'index.mdx'), 'utf8');
  const quoted = [...home.matchAll(/<pre><code>([\s\S]*?)<\/code><\/pre>/g)].map((match) => match[1].replace(/<[^>]+>/g, ''));
  assert.equal(quoted.length, 2, 'the card holds the table and the output');
  const fixture = join(pages, '..', '..', '..', 'examples', 'first-table');
  const sources = [readFileSync(join(fixture, 'archangel.config.ts'), 'utf8'), readFileSync(join(fixture, 'expected', 'check.txt'), 'utf8')];
  quoted.forEach((text, at) => {
    for (const line of text.split('\n').map((each) => each.trim()).filter(Boolean)) {
      assert.ok(sources[at].includes(line), `the card shows "${line}", which is not in the first table's ${at === 0 ? 'config' : 'output'}`);
    }
  });
});
