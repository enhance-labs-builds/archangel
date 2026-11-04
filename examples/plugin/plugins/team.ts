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
    return view.files
      .filter((file) => file.imports.length > options.max)
      .map((file) => ({
        file: file.path,
        message: `${file.path} has ${file.imports.length} imports, and the limit is ${options.max}`,
      }));
  },
});

export default definePlugin({ name: 'team', schemaVersion: 1, conditions: [maxImports] });
