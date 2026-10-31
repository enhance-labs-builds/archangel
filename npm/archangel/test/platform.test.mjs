// The launcher finds the platform package that matches the machine, or says what to do.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { chmodSync, cpSync, mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { PACKAGES, platformKey, unsupported, usesMusl } from '../lib/platform.js';

const here = dirname(dirname(fileURLToPath(import.meta.url)));

/** A copy of the package in a temporary folder, with the given platform packages installed. */
function install(withBinaryFor) {
  const root = mkdtempSync(join(tmpdir(), 'archangel-npm-'));
  const main = join(root, 'node_modules', '@enhance-eng', 'archangel');
  cpSync(here, main, { recursive: true, filter: (path) => !path.includes('/test') });
  if (withBinaryFor) {
    const bin = join(root, 'node_modules', withBinaryFor, 'bin');
    mkdirSync(bin, { recursive: true });
    const binary = join(bin, process.platform === 'win32' ? 'archangel.exe' : 'archangel');
    writeFileSync(binary, '#!/bin/sh\necho "the binary ran with $*"\nexit 3\n');
    chmodSync(binary, 0o755);
  }
  return join(main, 'bin', 'archangel.js');
}

const thisMachine = platformKey({ platform: process.platform, arch: process.arch, musl: usesMusl() });

test('unsupported platform names itself and the cargo alternative', () => {
  const key = platformKey({ platform: 'aix', arch: 'ppc64', musl: false });
  assert.equal(PACKAGES[key], undefined, 'aix has no package');
  const message = unsupported(key);
  assert.match(message, /aix-ppc64/, 'it names the platform');
  assert.match(message, /cargo install archangel/, 'it names the cargo alternative');
});

test('each supported platform has its own package', () => {
  const machines = [
    ['darwin', 'arm64', false],
    ['darwin', 'x64', false],
    ['linux', 'arm64', false],
    ['linux', 'arm64', true],
    ['linux', 'x64', false],
    ['linux', 'x64', true],
    ['win32', 'x64', false],
  ];
  const names = machines.map(([platform, arch, musl]) => PACKAGES[platformKey({ platform, arch, musl })]);
  assert.equal(new Set(names).size, 7, `seven packages: ${names}`);
  assert.ok(names.every((name) => name?.startsWith('@enhance-eng/archangel-')), `all scoped: ${names}`);
});

test('the launcher runs the binary of the machine with the arguments and its exit status', { skip: process.platform === 'win32' || !PACKAGES[thisMachine] }, () => {
  const launcher = install(PACKAGES[thisMachine]);
  const run = spawnSync(process.execPath, [launcher, 'check', '--stats'], { encoding: 'utf8' });
  assert.equal(run.stdout.trim(), 'the binary ran with check --stats', run.stderr);
  assert.equal(run.status, 3, 'the binary exit status passes through');
});

test('a missing platform package says how to install it', { skip: !PACKAGES[thisMachine] }, () => {
  const launcher = install(null);
  const run = spawnSync(process.execPath, [launcher, '--version'], { encoding: 'utf8' });
  assert.equal(run.status, 2, 'it stops with status 2');
  assert.match(run.stderr, new RegExp(PACKAGES[thisMachine].replace('/', '\\/')), 'it names the package');
  assert.match(run.stderr, /cargo install archangel/, 'and the cargo alternative');
});
