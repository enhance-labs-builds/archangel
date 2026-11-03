import { defineCondition, definePlugin } from '@enhance-eng/archangel';

const maxImports = defineCondition({
  name: 'max-imports',
  description: 'A file imports at most `max` other files.',
  reads: ['imports'],
  optionsSchema: {
    type: 'object',
    properties: { max: { type: 'integer', minimum: 1 } },
    required: ['max'],
  },
  check(view, options: { max: number }) {
    return view.files.flatMap((file) => {
      // A package is not another file of the repository, so only imports of files count.
      const files = file.imports.filter((each) => each.file !== null).length;
      return files > options.max
        ? [{ file: file.path, message: `${file.path} imports ${files} files, and the limit is ${options.max}` }]
        : [];
    });
  },
});

const noOrphans = defineCondition({
  name: 'no-orphans',
  description: 'Another file imports each file.',
  reads: ['importers'],
  check(view) {
    return view.files
      .filter((file) => file.importers.length === 0)
      .map((file) => ({ file: file.path, message: `no file imports ${file.path}; delete it or import it` }));
  },
});

export default definePlugin({ name: 'team', schemaVersion: 1, conditions: [maxImports, noOrphans] });
