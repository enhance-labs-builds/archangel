import { defineCondition, definePlugin } from '@enhance-eng/archangel';

const show = defineCondition({
  name: 'show',
  description: 'Prints what check receives, and reports nothing.',
  reads: ['imports', 'importers', 'exports'],
  optionsSchema: { type: 'object', properties: { max: { type: 'integer' } } },
  check(view, options) {
    console.log(JSON.stringify({ options, view }, null, 2));
    return [];
  },
});

export default definePlugin({ name: 'dump', schemaVersion: 1, conditions: [show] });
