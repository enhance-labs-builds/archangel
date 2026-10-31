// The config helper and the presets. `defineConfig` returns its argument: its only work is in the
// types, which check names, conditions and captures as the config is written.

/** @type {import('./index.d.ts').defineConfig} */
export const defineConfig = (config) => config;

export * from './plugin.js';
export { fsd } from './presets/fsd.js';
export { nextjs } from './presets/nextjs.js';
