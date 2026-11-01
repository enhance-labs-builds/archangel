// Run by archangel with Node: imports a config, answers with it as JSON, and then hosts the
// config's plugins until archangel closes standard input. The rule builders live here, not in the
// npm package, so the binary and the config cannot disagree about them.
//
// @justified informative
// The protocol is one JSON message per line. Node's first line is `{ type: 'config', config,
// files }`, with every file the config imported, or `{ type: 'error', error, files }`. Archangel then sends `rules` (each plugin rule with its condition and
// options), `file` (one file's record, and the plugin rules whose filters cover it), `remove`
// (a path), and `check` (the rules to run), which Node answers with one `result` line.
import { Console } from 'node:console';
import { registerHooks } from 'node:module';
import { createInterface } from 'node:readline';
import { Writable } from 'node:stream';
import { fileURLToPath, pathToFileURL } from 'node:url';

const SCHEMA_VERSION = 1;

// @justified warning
// Standard output carries the protocol, so everything the config or a plugin prints goes to
// standard error, with the name of the code that printed it.
const answer = process.stdout.write.bind(process.stdout);
const send = (message) => answer(`${JSON.stringify(message)}\n`);
let speaker = 'config';
const printed = new Writable({
  write(chunk, _encoding, done) {
    const lines = String(chunk).replace(/\n$/, '').split('\n');
    process.stderr.write(lines.map((line) => `[${speaker}] ${line}\n`).join(''));
    done();
  },
});
const printer = new Console({ stdout: printed, stderr: printed });
for (const method of ['log', 'info', 'warn', 'error', 'debug', 'trace']) {
  console[method] = printer[method].bind(printer);
}
process.stdout.write = (chunk) => printed.write(chunk);

const [configPath] = process.argv.slice(1);

// The files the config imports, its presets and plugins among them, so that a change to any of
// them loads the config again.
const imported = new Set();
registerHooks({
  resolve(specifier, context, nextResolve) {
    const resolved = nextResolve(specifier, context);
    if (resolved.url.startsWith('file:')) imported.add(fileURLToPath(resolved.url));
    return resolved;
  },
});
const files = () => [...imported];

const list = (value) => (Array.isArray(value) ? value : [value]);
const builders = {
  layers: (order, options = {}) => ({ ...options, kind: 'layers', order: list(order) }),
  isolate: (element, options = {}) => ({ ...options, kind: 'isolate', element }),
  entry: (element, files, options = {}) => ({ ...options, kind: 'entry', element, files: list(files) }),
  // @justified intent
  // A name first and a `name` key is two spellings of one key, which the parser refuses, so the
  // second name goes to the parser under a key that no config writes.
  rule: (first, body) =>
    typeof first === 'string'
      ? { ...body, name: first, ...(body?.name === undefined ? {} : { $name: body.name }) }
      : { ...first },
  // @justified intent
  // The removed rule kinds keep their builders, so that a config still calling one reaches the
  // parser, whose refusal gives the sentence to write instead.
  only: (element, importedBy, options = {}) => ({
    ...options,
    kind: 'only',
    element,
    importedBy: list(importedBy),
  }),
  forbid: (from, to, options = {}) => ({ ...options, kind: 'forbid', from: list(from), to: list(to) }),
  noCycles: (options = {}) => ({ ...options, kind: 'noCycles' }),
};

const isObject = (value) => value !== null && typeof value === 'object';

const CONDITION_FIELDS = ['name', 'description', 'docs', 'reads', 'optionsSchema', 'check'];

/** A plugin as archangel validates it: its schema version and its conditions, without code. */
const describe = (plugin) => ({
  schemaVersion: plugin?.schemaVersion ?? null,
  conditions: Array.isArray(plugin?.conditions)
    ? Object.fromEntries(
        plugin.conditions.map((condition) => [
          condition?.name,
          {
            description: condition?.description ?? '',
            docs: condition?.docs ?? null,
            reads: Array.isArray(condition?.reads) ? condition.reads : [],
            optionsSchema: condition?.optionsSchema ?? null,
            unknown: isObject(condition) ? Object.keys(condition).filter((key) => !CONDITION_FIELDS.includes(key)) : [],
          },
        ]),
      )
    : null,
});

/** A config or a preset's result, with its rules built and its plugins described. */
const part = (value) => {
  if (!isObject(value)) return value;
  const rules = typeof value.rules === 'function' ? value.rules(builders) : value.rules;
  const described = isObject(value.plugins)
    ? Object.fromEntries(Object.entries(value.plugins).map(([name, plugin]) => [name, describe(plugin)]))
    : value.plugins;
  return { ...value, rules, plugins: described };
};

const thrown = (error) => ({
  kind: 'thrown',
  code: typeof error?.code === 'string' ? error.code : null,
  message: error instanceof Error ? error.message : String(error),
});

let plugins = {};
try {
  const module = await import(pathToFileURL(configPath).href);
  if (!('default' in module)) {
    send({ type: 'error', error: { kind: 'no-default-export' }, files: files() });
  } else {
    const config = module.default;
    if (isObject(config)) {
      // @justified intent
      // Each preset's result goes to archangel unmerged, so that archangel can say which preset a
      // problem came from; the plugins of every preset are hosted beside the config's own.
      const presets = Array.isArray(config.extends) ? config.extends.map(part) : config.extends;
      plugins = Object.assign(
        {},
        ...(Array.isArray(config.extends) ? config.extends : []).map((p) => (isObject(p?.plugins) ? p.plugins : {})),
        isObject(config.plugins) ? config.plugins : {},
      );
      send({ type: 'config', config: { ...part(config), extends: presets }, files: files() });
    } else {
      send({ type: 'config', config, files: files() });
    }
  }
} catch (error) {
  send({ type: 'error', error: thrown(error), files: files() });
}
// Once the config has loaded, a line printed outside a check may come from any plugin.
const between = 'plugin process';
speaker = between;

/** The plugin rules by index, each with its condition and options. */
const rules = new Map();
/** Each file's record, and the plugin rules whose filters cover it, by path. */
const records = new Map();

/** A record with only the facts a condition reads. */
const project = (record, reads) => {
  const { imports, importers, exports, ...always } = record;
  const seen = { ...always };
  if (reads.includes('imports')) seen.imports = imports ?? [];
  if (reads.includes('importers')) seen.importers = importers ?? [];
  if (reads.includes('exports')) seen.exports = exports ?? { names: [], default: false, star: false };
  return seen;
};

const run = async (index) => {
  const rule = rules.get(index);
  const condition = plugins[rule?.plugin]?.conditions?.find((c) => c?.name === rule.condition);
  if (typeof condition?.check !== 'function') {
    return { rule: index, error: `the condition \`${rule?.plugin}/${rule?.condition}\` has no check function` };
  }
  speaker = rule.plugin;
  try {
    const reads = Array.isArray(condition.reads) ? condition.reads : [];
    const files = [...records.values()]
      .filter((entry) => entry.rules.includes(index))
      .map((entry) => project(entry.record, reads))
      .sort((a, b) => (a.path < b.path ? -1 : a.path > b.path ? 1 : 0));
    const found = await condition.check({ schemaVersion: SCHEMA_VERSION, files }, rule.options);
    return { rule: index, violations: Array.isArray(found) ? found : [] };
  } catch (error) {
    return { rule: index, error: error instanceof Error ? error.message : String(error) };
  } finally {
    speaker = between;
  }
};

for await (const line of createInterface({ input: process.stdin, crlfDelay: Infinity })) {
  const message = JSON.parse(line);
  if (message.type === 'rules') {
    rules.clear();
    for (const rule of message.rules) rules.set(rule.rule, rule);
  } else if (message.type === 'file') {
    records.set(message.record.path, { rules: message.rules, record: message.record });
  } else if (message.type === 'remove') {
    records.delete(message.path);
  } else if (message.type === 'check') {
    const results = [];
    for (const index of message.rules) results.push(await run(index));
    send({ type: 'result', id: message.id, rules: results });
  }
}
// A write to a pipe can still be queued when the input closes, so the process exits once standard
// error has taken everything printed.
process.stderr.write('', () => process.exit(0));
