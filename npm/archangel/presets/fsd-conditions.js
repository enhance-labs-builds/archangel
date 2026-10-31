// The `fsd` plugin: the seven Feature-Sliced Design rules that are guesses about names or counts,
// which serve this one preset and so stay out of archangel's built-in conditions. Each follows
// the rule of the same name in Steiger 0.8.1's Feature-Sliced Design plugin, over a folder tree
// built from the paths in the view, and reports the folder Steiger reports.
import pluralize from 'pluralize';

import { defineCondition, definePlugin } from '../plugin.js';

const LAYERS = ['shared', 'entities', 'features', 'widgets', 'pages', 'app'];
const UNSLICED = ['shared', 'app'];
const SEGMENTS = ['ui', 'api', 'lib', 'model', 'config'];
const SOURCE = ['.tsx', '.jsx', '.ts', '.js', '.cjs', '.mjs'];
const DOCS = 'https://feature-sliced.github.io/steiger/rules/';

const baseName = (path) => path.slice(path.lastIndexOf('/') + 1);

/** A name without its last extension, as Node's `path.parse` gives it. */
const stem = (path) => {
  const base = baseName(path);
  const dot = base.lastIndexOf('.');
  return dot > 0 ? base.slice(0, dot) : base;
};

/** Two paths joined with a slash, either of which may be empty. */
const join = (first, second) => (first === '' ? second : second === '' ? first : `${first}/${second}`);

/** The folder tree below `root`, from the paths of the view's files. */
const tree = (view, root) => {
  const top = { type: 'folder', path: root, children: [] };
  const folders = new Map([[root, top]]);
  const prefix = root === '' ? '' : `${root}/`;
  for (const file of view.files) {
    if (!file.path.startsWith(prefix)) continue;
    const parts = file.path.slice(prefix.length).split('/');
    let folder = top;
    for (const part of parts.slice(0, -1)) {
      const path = join(folder.path, part);
      let next = folders.get(path);
      if (next === undefined) {
        next = { type: 'folder', path, children: [] };
        folders.set(path, next);
        folder.children.push(next);
      }
      folder = next;
    }
    folder.children.push({ type: 'file', path: file.path });
  }
  return top;
};

const hasPrefix = (name) => /^([0-9]_|_)/.test(name);
const removePrefix = (name) => name.replace(/^([0-9]_|_)/, '');

const layersOf = (root) => {
  const found = [];
  for (const child of root.children) {
    if (child.type !== 'folder') continue;
    const name = baseName(child.path);
    const layer = LAYERS.find((l) => l === removePrefix(name));
    if (layer === undefined) continue;
    const existing = found.find(([l]) => l === layer);
    if (existing === undefined) found.push([layer, child]);
    else if (hasPrefix(name) && !hasPrefix(baseName(existing[1].path))) existing[1] = child;
  }
  return Object.fromEntries(found);
};

const isSliced = (layer) => !UNSLICED.includes(removePrefix(baseName(typeof layer === 'string' ? layer : layer.path)));
const isIndex = (node) => node.type === 'file' && stem(node.path).split('.')[0] === 'index';
const isSlice = (folder) => folder.children.some((child) => SEGMENTS.includes(stem(child.path)));
const indexesOf = (node) => (node.type === 'file' ? [node] : node.children.filter(isIndex));
const segmentsOf = (folder) =>
  Object.fromEntries(folder.children.filter((child) => !isIndex(child)).map((child) => [stem(child.path), child]));

const slicesOf = (layer) => {
  const slices = {};
  const traverse = (folder, prefix) => {
    if (isSlice(folder)) {
      slices[join(prefix, baseName(folder.path))] = folder;
      return;
    }
    for (const child of folder.children) {
      if (child.type === 'folder') traverse(child, join(prefix, baseName(folder.path)));
    }
  };
  for (const child of layer.children) if (child.type === 'folder') traverse(child, '');
  return slices;
};

const allSlices = (root) => {
  const found = {};
  for (const [layerName, layer] of Object.entries(layersOf(root))) {
    if (!isSliced(layer)) continue;
    for (const [name, slice] of Object.entries(slicesOf(layer))) found[name] = { ...slice, layerName };
  }
  return found;
};

const groupSlices = (names) => {
  const groups = {};
  for (const name of names) {
    const parts = name.split('/');
    const last = parts.pop();
    const group = parts.join('/');
    (groups[group] ??= []).push(last);
  }
  return groups;
};

/** Each source file in a layer, with its layer and slice, as Steiger indexes them. */
const indexSourceFiles = (root) => {
  const index = {};
  const walk = (node, place) => {
    if (node.type === 'file') index[node.path] = place;
    else for (const child of node.children) walk(child, place);
  };
  for (const [layerName, layer] of Object.entries(layersOf(root))) {
    for (const child of layer.children) {
      if (child.type === 'file') walk(child, { layerName, sliceName: null });
    }
    if (!isSliced(layer)) {
      for (const segment of Object.values(segmentsOf(layer))) walk(segment, { layerName, sliceName: null });
    } else {
      for (const [sliceName, slice] of Object.entries(slicesOf(layer))) {
        for (const segment of Object.values(segmentsOf(slice))) walk(segment, { layerName, sliceName });
        for (const index2 of indexesOf(slice)) walk(index2, { layerName, sliceName });
      }
    }
  }
  return index;
};

/** The Levenshtein distance between two names. */
const distance = (a, b) => {
  let previous = Array.from({ length: b.length + 1 }, (_, i) => i);
  for (let i = 1; i <= a.length; i += 1) {
    const current = [i];
    for (let j = 1; j <= b.length; j += 1) {
      const cost = a[i - 1] === b[j - 1] ? 0 : 1;
      current.push(Math.min(previous[j] + 1, current[j - 1] + 1, previous[j - 1] + cost));
    }
    previous = current;
  }
  return previous[b.length];
};

const rootOption = {
  type: 'object',
  properties: { root: { type: 'string', description: 'The Feature-Sliced Design root.' } },
  required: ['root'],
  additionalProperties: false,
};

const condition = (name, description, reads, check) =>
  defineCondition({ name, description, docs: `${DOCS}${name}`, options: rootOption, reads, check });

const typoInLayerName = condition(
  'typo-in-layer-name',
  'A folder in the root whose name is one to three edits from a layer name is probably that layer, misspelt.',
  [],
  (view, { root }) => {
    const violations = [];
    for (const child of tree(view, root).children) {
      if (child.type !== 'folder') continue;
      const name = baseName(child.path);
      if (LAYERS.includes(name)) continue;
      const near = LAYERS.map((layer) => [distance(name, layer), layer])
        .filter(([d]) => d <= 3)
        .sort((a, b) => a[0] - b[0])[0];
      if (near !== undefined) {
        violations.push({ file: child.path, message: `Layer "${name}" potentially contains a typo. Did you mean "${near[1]}"?` });
      }
    }
    return violations;
  },
);

const ambiguousSliceNames = condition(
  'ambiguous-slice-names',
  'A slice or slice group may not be named like a segment of Shared.',
  [],
  (view, { root }) => {
    const top = tree(view, root);
    const layers = layersOf(top);
    if (layers.shared === undefined) return [];
    const segmentNames = Object.keys(segmentsOf(layers.shared));
    const violations = [];
    for (const [sliceName, slice] of Object.entries(allSlices(top))) {
      const parts = sliceName.split('/');
      const matching = segmentNames.find((segment) => parts.includes(segment));
      if (matching === undefined) continue;
      if (matching === sliceName) {
        violations.push({ file: slice.path, message: `Slice "${sliceName}" could be confused with a segment from Shared with the same name` });
        continue;
      }
      const at = parts.indexOf(matching);
      if (at === parts.length - 1) {
        violations.push({ file: slice.path, message: `Slice "${sliceName}" could be confused with a segment "${matching}" from Shared` });
        continue;
      }
      const toGroup = parts.slice(0, at + 1);
      const group = toGroup.reduce(
        (folder, part) => folder?.children.find((child) => child.type === 'folder' && baseName(child.path) === part),
        layers[slice.layerName],
      );
      if (group !== undefined) {
        violations.push({ file: group.path, message: `Slice group "${toGroup.join('/')}" could be confused with a segment "${matching}" from Shared` });
      }
    }
    return violations;
  },
);

const NEUTRAL = new Set(['k8s', 'kubernetes', 'media']);

const inconsistentNaming = condition(
  'inconsistent-naming',
  'The slices of Entities in one group are all plural or all singular.',
  [],
  (view, { root }) => {
    const { entities } = layersOf(tree(view, root));
    if (entities === undefined) return [];
    const violations = [];
    for (const [group, names] of Object.entries(groupSlices(Object.keys(slicesOf(entities))))) {
      const checked = names.filter((name) => !NEUTRAL.has(name.toLowerCase()));
      const plural = checked.filter((name) => pluralize.isPlural(name));
      const singular = checked.filter((name) => !pluralize.isPlural(name));
      if (plural.length > 0 && singular.length > 0) {
        const prefer = plural.length >= singular.length ? 'plural' : 'singular';
        violations.push({
          file: join(entities.path, group),
          message: `Inconsistent pluralization of slice names. Prefer all ${prefer} names`,
        });
      }
    }
    return violations;
  },
);

const insignificantSlice = condition(
  'insignificant-slice',
  'A slice outside Pages is used by at least two other slices or layers.',
  ['imports'],
  (view, { root }) => {
    const top = tree(view, root);
    const index = indexSourceFiles(top);
    const imports = new Map(view.files.map((file) => [file.path, file.imports]));
    const references = new Map();
    const referenced = (key) => {
      if (!references.has(key)) references.set(key, new Set());
      return references.get(key);
    };
    for (const [path, place] of Object.entries(index)) {
      const key = join(place.layerName, place.sliceName ?? '');
      const extension = path.slice(path.lastIndexOf('.'));
      const found = SOURCE.includes(extension) ? (imports.get(path) ?? []) : [];
      for (const target of found.map((i) => (i.file === null ? undefined : index[i.file]))) {
        if (target === undefined || target.layerName === place.layerName) continue;
        const targetKey = join(target.layerName, target.sliceName ?? '');
        if (targetKey !== key) referenced(targetKey).add(key);
      }
      referenced(key);
    }
    const violations = [];
    for (const [key, users] of references) {
      const layer = key.split('/')[0];
      if (!isSliced(layer) || layer === 'pages') continue;
      const file = join(top.path, key);
      if (users.size === 1) {
        const [user] = users;
        if (UNSLICED.includes(user)) {
          if (user !== 'app') {
            violations.push({ file, message: `This slice has only one reference on layer "${user}". Consider moving this code to "${user}".` });
          }
        } else {
          violations.push({ file, message: `This slice has only one reference in slice "${user}". Consider merging them.` });
        }
      } else if (users.size === 0) {
        violations.push({ file, message: 'This slice has no references. Consider removing it.' });
      }
    }
    return violations;
  },
);

const repetitiveNaming = condition(
  'repetitive-naming',
  'The slices of a layer or group do not all repeat one word.',
  [],
  (view, { root }) => {
    const violations = [];
    for (const layer of Object.values(layersOf(tree(view, root)))) {
      if (!isSliced(layer)) continue;
      for (const [group, names] of Object.entries(groupSlices(Object.keys(slicesOf(layer))))) {
        const words = names.map((name) => (name.match(/(?:[A-Z]+|[a-z]+)[a-z]*/g) ?? []).map((w) => w.toLowerCase()));
        const counts = new Map();
        for (const word of words.flat()) counts.set(word, (counts.get(word) ?? 0) + 1);
        for (const [word, count] of counts) {
          if (names.length > 2 && count >= names.length && words.every((w) => w.includes(word))) {
            violations.push({ file: join(layer.path, group), message: `Repetitive word "${word}" in slice names.` });
          }
        }
      }
    }
    return violations;
  },
);

const excessiveSlicing = condition(
  'excessive-slicing',
  'A sliced layer, or a group in one, has at most 20 slices.',
  [],
  (view, { root }) => {
    const violations = [];
    for (const [layerName, layer] of Object.entries(layersOf(tree(view, root)))) {
      if (!isSliced(layer)) continue;
      for (const [group, names] of Object.entries(groupSlices(Object.keys(slicesOf(layer))))) {
        if (names.length <= 20) continue;
        const where = group === '' ? `Layer "${layerName}" has ${names.length} ungrouped slices` : `Slice group "${group}" has ${names.length} slices`;
        violations.push({
          file: join(layer.path, group),
          message: `${where}, which is above the recommended threshold of 20. Consider grouping them or moving the code inside to the layer where it's used.`,
        });
      }
    }
    return violations;
  },
);

const sharedLibGrouping = condition(
  'shared-lib-grouping',
  'Shared/lib holds at most 15 modules.',
  [],
  (view, { root }) => {
    const { shared } = layersOf(tree(view, root));
    const lib = shared === undefined ? undefined : segmentsOf(shared).lib;
    if (lib?.type !== 'folder' || lib.children.length <= 15) return [];
    return [
      {
        file: lib.path,
        message: `Shared/lib has ${lib.children.length} modules, which is above the recommended threshold of 15. Consider grouping them.`,
      },
    ];
  },
);

/** The plugin the `fsd` preset brings, which its rules use as `fsd/<condition>`. */
export const fsdPlugin = definePlugin({
  name: 'fsd',
  schemaVersion: 1,
  conditions: [
    typoInLayerName,
    ambiguousSliceNames,
    inconsistentNaming,
    insignificantSlice,
    repetitiveNaming,
    excessiveSlicing,
    sharedLibGrouping,
  ],
});
