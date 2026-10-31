import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

const script = fileURLToPath(new URL('release-version.mjs', import.meta.url));

/** A repository whose files carry `version`, with the darwin-arm64 package at `platform`. */
const repository = (version, platform = version) => {
  const root = mkdtempSync(join(tmpdir(), 'archangel-release-'));
  const write = (path, text) => {
    mkdirSync(join(root, path, '..'), { recursive: true });
    writeFileSync(join(root, path), text);
  };
  write(
    'Cargo.toml',
    `[workspace.package]\nversion = "${version}"\n\n[workspace.dependencies]\n` +
      `archangel_config = { path = "crates/archangel_config", version = "${version}" }\n`,
  );
  write(
    'npm/archangel/package.json',
    JSON.stringify({ version, optionalDependencies: { '@enhance-eng/archangel-darwin-arm64': version } }),
  );
  write('npm/platforms/darwin-arm64/package.json', JSON.stringify({ version: platform }));
  write('npm/platforms/README.md', 'Not a package.\n');
  write('editors/vscode/package.json', JSON.stringify({ version }));
  return root;
};

const run = (tag, root) => spawnSync(process.execPath, [script, tag, root], { encoding: 'utf8' });

test('a tag that names every version passes', () => {
  const found = run('v0.1.0', repository('0.1.0'));
  assert.equal(found.status, 0, found.stderr);
  assert.equal(found.stdout, 'every version is 0.1.0\n');
});

test('a tag that names another version fails and lists each place to change', () => {
  const found = run('v0.2.0', repository('0.1.0'));
  assert.equal(found.status, 1);
  assert.match(found.stderr, /every version must be 0\.2\.0, the tag v0\.2\.0's/);
  assert.match(found.stderr, /Cargo\.toml: \[workspace\.package\] version: 0\.1\.0/);
  assert.match(found.stderr, /Cargo\.toml: the archangel_config dependency: 0\.1\.0/);
  assert.match(found.stderr, /npm\/platforms\/darwin-arm64\/package\.json: version: 0\.1\.0/);
  assert.match(found.stderr, /Set every version in one commit, then tag that commit/);
});

test('without a tag, every version must be the workspace version', () => {
  const found = run('', repository('0.1.0', '0.0.9'));
  assert.equal(found.status, 1);
  assert.match(found.stderr, /every version must be 0\.1\.0, the workspace's/);
  assert.match(found.stderr, /npm\/platforms\/darwin-arm64\/package\.json: version: 0\.0\.9/);
  assert.doesNotMatch(found.stderr, /Cargo\.toml/);
});
