// The plugin helpers. Each returns its argument: its only work is in the types, which check a
// condition's options and the facts it reads as the plugin is written.

/** @type {import('./plugin.d.ts').defineCondition} */
export const defineCondition = (condition) => condition;

/** @type {import('./plugin.d.ts').definePlugin} */
export const definePlugin = (plugin) => plugin;
