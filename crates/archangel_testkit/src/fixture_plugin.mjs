// The plugin that archangel's tests load as `fixture`: conditions that report imports, count
// imports or importers, throw, stop their own process, report a file outside their view, describe
// the records they receive, write a line straight to standard output, print as their input closes,
// and wait on a gate that the test opens.
import { writeSync } from 'node:fs';
import { connect } from 'node:net';

const condition = (name, reads, optionsSchema, check) => ({
  name,
  description: `The fixture condition ${name}.`,
  docs: `https://example.com/fixture/${name}`,
  reads,
  optionsSchema,
  check,
});

const object = (properties, required = Object.keys(properties)) => ({
  type: 'object',
  properties,
  required,
  additionalProperties: false,
});

const importsOf = (view, specifier) =>
  view.files.flatMap((file) =>
    file.imports
      .filter((found) => found.specifier === specifier)
      .map((found) => ({
        file: file.path,
        import: { specifier: found.specifier, occurrence: found.occurrence },
        message: `${file.path} imports ${specifier}`,
      })),
  );

// @justified intent
// The gate is a connection to the test's listener, which closes it when the test opens the gate,
// so the wait ends on the test's action and never on a duration.
const gate = (port) =>
  new Promise((resolve, reject) => {
    const socket = connect(port, '127.0.0.1');
    socket.on('close', resolve);
    socket.on('error', reject);
    socket.resume();
  });

export default {
  name: 'fixture',
  schemaVersion: 1,
  conditions: [
    condition(
      'report-imports',
      ['imports'],
      object({ specifier: { type: 'string' } }),
      (view, { specifier }) => importsOf(view, specifier),
    ),
    condition('count-imports', ['imports'], object({ max: { type: 'number' } }), (view, { max }) =>
      view.files
        .filter((file) => file.imports.length > max)
        .map((file) => ({
          file: file.path,
          message: `${file.path} has ${file.imports.length} imports, more than ${max}`,
        })),
    ),
    condition(
      'importers-at-most',
      ['importers'],
      object({ max: { type: 'number' } }),
      (view, { max }) =>
        view.files
          .filter((file) => file.importers.length > max)
          .map((file) => ({
            file: file.path,
            message: `${file.path} has ${file.importers.length} importers, more than ${max}`,
          })),
    ),
    condition('throws', [], object({ message: { type: 'string' } }), (_view, { message }) => {
      throw new Error(message);
    }),
    condition('exits', [], null, () => {
      console.log('the fixture stops its process');
      process.exit(3);
    }),
    condition('reports-outside', [], object({ file: { type: 'string' } }), (_view, { file }) => [
      { file, message: `${file} is not in the view` },
    ]),
    condition('describes', ['imports', 'importers', 'exports'], null, (view) =>
      view.files.map((file) => ({ file: file.path, message: Object.keys(file).sort().join(' ') })),
    ),
    condition(
      'writes-to-stdout',
      ['imports'],
      object({ specifier: { type: 'string' } }),
      (view, { specifier }) => {
        writeSync(1, 'a line that is not the answer\n');
        return importsOf(view, specifier);
      },
    ),
    condition('prints-when-closed', [], null, () => {
      process.stdin.once('end', () => console.log('the fixture prints as its input closes'));
      return [];
    }),
    condition(
      'waits',
      ['imports'],
      object({ port: { type: 'integer' }, specifier: { type: 'string' } }),
      async (view, { port, specifier }) => {
        const found = importsOf(view, specifier);
        if (found.length > 0) {
          await gate(port);
        }
        return found;
      },
    ),
  ],
};
