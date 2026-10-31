#!/usr/bin/env node
// Runs the archangel binary from the platform package that matches this machine.
import { spawnSync } from 'node:child_process';
import { createRequire } from 'node:module';
import { PACKAGES, missing, platformKey, unsupported, usesMusl } from '../lib/platform.js';

const key = platformKey({ platform: process.platform, arch: process.arch, musl: usesMusl() });
const name = PACKAGES[key];
if (name === undefined) {
  process.stderr.write(`${unsupported(key)}\n`);
  process.exit(2);
}

const binary = process.platform === 'win32' ? 'bin/archangel.exe' : 'bin/archangel';
let path;
try {
  path = createRequire(import.meta.url).resolve(`${name}/${binary}`);
} catch {
  process.stderr.write(`${missing(key, name)}\n`);
  process.exit(2);
}

const run = spawnSync(path, process.argv.slice(2), { stdio: 'inherit' });
if (run.error) {
  process.stderr.write(`archangel could not start ${path}: ${run.error.message}\n`);
  process.exit(2);
}
process.exit(run.status ?? 2);
