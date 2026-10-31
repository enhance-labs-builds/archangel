# config-load-failed

The config's code threw while it loaded, has no default export, or uses TypeScript that Node cannot strip, such as an enum.

## Example

```ts
enum Layer { App, Shared } // Node cannot strip an enum
export default defineConfig({ ... });
```

## What to do

Fix the error the message names. Use a constant object with `as const` instead of an enum, add `.ts` to relative imports, and give the file a default export.
