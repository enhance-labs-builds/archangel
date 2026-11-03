// Runs the published archangel on a copy of a fixture, as a reader who installed it would.
import { spawnSync } from 'node:child_process';
import { cpSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, symlinkSync, writeFileSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

export const site = join(dirname(fileURLToPath(import.meta.url)), '..');
export const examples = join(site, 'examples');
export const pages = join(site, 'src', 'content', 'docs');
export const archangel = join(site, 'node_modules', '@enhance-eng', 'archangel');

/** Set UPDATE=1 to write what archangel prints into each fixture's expected output. */
export const updating = process.env.UPDATE === '1';

/** The name of each fixture, or only those that FIXTURE names, split by commas. */
export function fixtureNames() {
  const only = process.env.FIXTURE?.split(',');
  return readdirSync(examples, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .filter((name) => only === undefined || only.includes(name))
    .sort();
}

/** Writes the published package, and an empty package for each other dependency of the manifest. */
function install(folder) {
  const scope = join(folder, 'node_modules', '@enhance-eng');
  mkdirSync(scope, { recursive: true });
  symlinkSync(archangel, join(scope, 'archangel'), 'junction');
  const manifest = join(folder, 'package.json');
  if (!existsSync(manifest)) return;
  const { dependencies = {}, devDependencies = {}, workspaces = [] } = JSON.parse(readFileSync(manifest, 'utf8'));
  // A workspace package is linked, as npm links it, so its imports resolve to its files.
  const linked = new Set();
  for (const pattern of workspaces) {
    const parent = join(folder, pattern.replace(/\/\*$/, ''));
    for (const entry of readdirSync(parent, { withFileTypes: true }).filter((each) => each.isDirectory())) {
      const at = join(parent, entry.name);
      const { name } = JSON.parse(readFileSync(join(at, 'package.json'), 'utf8'));
      mkdirSync(dirname(join(folder, 'node_modules', name)), { recursive: true });
      symlinkSync(at, join(folder, 'node_modules', name), 'junction');
      linked.add(name);
      const own = JSON.parse(readFileSync(join(at, 'package.json'), 'utf8'));
      Object.assign(dependencies, own.dependencies ?? {}, own.devDependencies ?? {});
    }
  }
  for (const name of Object.keys({ ...dependencies, ...devDependencies })) {
    if (name === '@enhance-eng/archangel' || linked.has(name)) continue;
    const at = join(folder, 'node_modules', name);
    mkdirSync(at, { recursive: true });
    writeFileSync(join(at, 'package.json'), JSON.stringify({ name, main: 'index.js' }));
    writeFileSync(join(at, 'index.js'), '');
  }
}

/**
 * The folders of a fixture that are not part of its repository: `expected/` holds the documented
 * output, `later/` holds the files that a later step writes, and `before/` holds the config of the
 * tool that a migration replaces.
 */
const OUTSIDE = ['expected', 'later', 'before'];

/** A copy of a folder of files, without its expected output, with its packages installed. */
export function copyOf(folder) {
  const copy = mkdtempSync(join(tmpdir(), 'archangel-site-'));
  cpSync(folder, copy, {
    recursive: true,
    filter: (path) => !OUTSIDE.includes(relative(folder, path).split(/[\\/]/)[0]),
  });
  install(copy);
  return copy;
}

/** A copy of the files given by path, with their packages installed. */
export function repoOf(files) {
  const copy = mkdtempSync(join(tmpdir(), 'archangel-site-'));
  for (const [path, text] of Object.entries(files)) {
    mkdirSync(dirname(join(copy, path)), { recursive: true });
    writeFileSync(join(copy, path), text);
  }
  install(copy);
  return copy;
}

/** Runs the published archangel in a folder, with no colour. */
export function run(folder, args) {
  const found = spawnSync(process.execPath, [join(archangel, 'bin', 'archangel.js'), ...args, '--color', 'never'], {
    cwd: folder,
    encoding: 'utf8',
    env: { ...process.env, NO_COLOR: '1' },
  });
  return { status: found.status, stdout: found.stdout, stderr: found.stderr };
}

/**
 * The steps of a fixture: `expected/commands.json` when it has one, or else `check`, `test` and
 * `rules`. A step runs a command, and keeps its output and its exit status in `expected/`. A step
 * can also keep a file that the command writes.
 */
export function stepsOf(name) {
  const listed = join(examples, name, 'expected', 'commands.json');
  if (existsSync(listed)) return JSON.parse(readFileSync(listed, 'utf8'));
  return ['check', 'test', 'rules'].map((command) => ({ args: [command], output: `${command}.txt` }));
}

/** Why an output differs from the documented output: the first line that differs. */
export function difference(fixture, command, got, expected) {
  if (got === expected) return null;
  const a = got.split('\n');
  const b = expected.split('\n');
  const at = Array.from({ length: Math.max(a.length, b.length) }, (_, i) => i).find((i) => a[i] !== b[i]);
  return `the fixture ${fixture}: \`archangel ${command}\` differs at line ${at + 1}: the page says ${JSON.stringify(b[at])}, and archangel prints ${JSON.stringify(a[at])}`;
}

/** The output that a step shows on a page: what archangel prints, and its exit status. */
export function shown(found) {
  return found.stdout + found.stderr;
}
