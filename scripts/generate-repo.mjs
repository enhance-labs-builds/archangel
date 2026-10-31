#!/usr/bin/env node
// Writes a synthetic repository for the benchmark: `files` TypeScript files of `lines` lines each,
// in three layers of features, where each file imports a few files of its own feature and below.
// Usage: generate-repo.mjs <folder> <files> <lines>
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const [folder, files, lines] = process.argv.slice(2);
const count = Number(files);
const length = Number(lines);
const layers = ['app', 'features', 'shared'];
const perFeature = 50;

const place = (index) => {
  const layer = layers[index % layers.length];
  const feature = Math.floor(index / (layers.length * perFeature));
  return { layer, feature, path: `src/${layer}/f${feature}/m${index}.ts` };
};

for (let index = 0; index < count; index += 1) {
  const here = place(index);
  const imports = [1, 2, 3]
    .map((step) => index - step * layers.length)
    .filter((other) => other >= 0)
    .map(place)
    .filter((other) => layers.indexOf(other.layer) >= layers.indexOf(here.layer))
    .map((other) => `import { value as v${other.path.length} } from '../../${other.layer}/f${other.feature}/m${other.path.split('/m')[1].replace('.ts', '')}';`);
  const body = Array.from({ length: Math.max(0, length - imports.length - 1) }, (_, line) =>
    `export const value${line} = ${line} * ${index};`,
  );
  const text = [...new Set(imports), ...body, `export const value = ${index};`].join('\n');
  mkdirSync(join(folder, `src/${here.layer}/f${here.feature}`), { recursive: true });
  writeFileSync(join(folder, here.path), `${text}\n`);
}
writeFileSync(
  join(folder, 'archangel.config.json'),
  JSON.stringify({
    elements: { app: 'src/app/**', features: 'src/features/{feature}/**', shared: 'src/shared/**' },
    rules: [{ kind: 'layers', order: layers }, { files: '**', noCycles: true }],
  }),
);
