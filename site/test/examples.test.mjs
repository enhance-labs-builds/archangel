// Spec: openspec/changes/documentation-site, "Each fixture gives its documented output with the
// published release".
import assert from 'node:assert/strict';
import { existsSync, lstatSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { test } from 'node:test';
import { archangel, copyOf, difference, examples, fixtureNames, repoOf, run, shown, site, stepsOf, updating } from './support.mjs';

/** Runs each step of a fixture, and gives each difference from its documented output. */
function differences(name) {
  const copy = copyOf(join(examples, name));
  const expected = join(examples, name, 'expected');
  const found = [];
  for (const step of stepsOf(name)) {
    // A step can change the files first, as a developer does between two commands.
    // `later` names a file of the fixture's `later/` folder, which a page can show.
    const writes = Object.entries(step.write ?? {}).concat(
      Object.entries(step.later ?? {}).map(([path, file]) => [path, readFileSync(join(examples, name, 'later', file), 'utf8')]),
    );
    for (const [path, text] of writes) {
      mkdirSync(dirname(join(copy, path)), { recursive: true });
      writeFileSync(join(copy, path), text);
    }
    if (!step.args) continue;
    const result = run(copy, step.args);
    const command = step.args.join(' ');
    if (step.status !== undefined && result.status !== step.status) {
      found.push(`the fixture ${name}: \`archangel ${command}\` exits with ${result.status}, and the page says ${step.status}\n${shown(result)}`);
    }
    const kept = [];
    if (step.output) kept.push([join(expected, step.output), shown(result)]);
    for (const [written, file] of Object.entries(step.files ?? {})) {
      kept.push([join(expected, file), readFileSync(join(copy, written), 'utf8')]);
    }
    for (const [file, got] of kept) {
      if (updating) writeFileSync(file, got);
      const text = existsSync(file) ? readFileSync(file, 'utf8') : '';
      const why = difference(name, command, got, text);
      if (why) found.push(why);
    }
  }
  return found;
}

test('an output that differs is named', () => {
  const repo = repoOf({
    'package.json': '{ "name": "a", "private": true }\n',
    'archangel.config.json':
      '{ "parts": { "a": "src/a/**", "b": "src/b/**" }, "rules": [{ "kind": "mayImport", "name": "t", "rows": { "a": [], "b": [] } }] }\n',
    'src/a/x.ts': "import '../b/y';\n",
    'src/b/y.ts': '',
  });
  const found = run(repo, ['check']);
  assert.equal(found.status, 1, shown(found));
  const why = difference('refused', 'check', shown(found), 'no problem\n');
  assert.match(why, /^the fixture refused: `archangel check` differs at line 1: the page says "no problem"/);
});

test('each fixture gives its documented output', () => {
  const found = fixtureNames().flatMap(differences);
  assert.deepEqual(found, [], found.join('\n\n'));
});

test('the site runs the published package', () => {
  const manifest = JSON.parse(readFileSync(join(site, 'package.json'), 'utf8'));
  const pinned = manifest.devDependencies['@enhance-eng/archangel'];
  assert.match(pinned, /^\d+\.\d+\.\d+$/, 'the site pins one exact release');
  assert.ok(!lstatSync(archangel).isSymbolicLink(), 'the package is installed, not linked to this repository');
  const installed = JSON.parse(readFileSync(join(archangel, 'package.json'), 'utf8'));
  assert.equal(installed.version, pinned, 'the installed package is the pinned release');
});

test('each expected file is the output of a step', () => {
  const found = fixtureNames().flatMap((name) => {
    const made = new Set(['commands.json']);
    for (const step of stepsOf(name)) {
      if (step.output) made.add(step.output);
      for (const file of Object.values(step.files ?? {})) made.add(file);
    }
    return readdirSync(join(examples, name, 'expected'))
      .filter((file) => !made.has(file))
      .map((file) => `examples/${name}/expected/${file} is the output of no step, so no test runs it`);
  });
  assert.deepEqual(found, [], found.join('\n'));
});
