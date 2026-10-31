#!/usr/bin/env node
// Every version a release publishes must be the same: the crates', the npm packages' and the
// extension's. Given a tag such as v0.1.0, it must also be the tag's, so that a tag never publishes
// a version it does not name.
import { readdirSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const root = process.argv[3] ?? fileURLToPath(new URL('..', import.meta.url));
const read = (path) => readFileSync(`${root}/${path}`, 'utf8');
const json = (path) => JSON.parse(read(path));

const found = new Map();
const cargo = read('Cargo.toml');
found.set('Cargo.toml: [workspace.package] version', /\[workspace\.package\][^[]*?\nversion = "([^"]+)"/.exec(cargo)?.[1]);
for (const [, crate, version] of cargo.matchAll(/^(archangel_\w+) = \{[^}]*version = "([^"]+)"/gm)) {
  found.set(`Cargo.toml: the ${crate} dependency`, version);
}
const main = json('npm/archangel/package.json');
found.set('npm/archangel/package.json: version', main.version);
for (const [name, version] of Object.entries(main.optionalDependencies ?? {})) {
  found.set(`npm/archangel/package.json: optionalDependencies ${name}`, version);
}
for (const entry of readdirSync(`${root}/npm/platforms`, { withFileTypes: true })) {
  if (!entry.isDirectory()) continue;
  const platform = entry.name;
  found.set(`npm/platforms/${platform}/package.json: version`, json(`npm/platforms/${platform}/package.json`).version);
}
found.set('editors/vscode/package.json: version', json('editors/vscode/package.json').version);

const tag = process.argv[2] || undefined;
const wanted = tag ? tag.replace(/^v/, '') : found.get('Cargo.toml: [workspace.package] version');
const differ = [...found].filter(([, version]) => version !== wanted);
if (differ.length > 0) {
  const whose = tag ? `the tag ${tag}'s` : "the workspace's";
  console.error(
    `every version must be ${wanted}, ${whose}, and these are not:\n` +
      differ.map(([place, version]) => `  ${place}: ${version ?? 'missing'}`).join('\n') +
      '\nSet every version in one commit, then tag that commit. See docs/releasing.md.',
  );
  process.exit(1);
}
console.log(`every version is ${wanted}`);
