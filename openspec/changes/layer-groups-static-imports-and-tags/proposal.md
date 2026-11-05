# Layers of groups, rules that stop at `import()`, and a selector for a docblock tag

## Why

The same team asked for three things that their ESLint setup did, or could not do:

- **#47.** A file can be in one part only. A file in `src/features/billing/` is in the part
  `feature`, whose capture an `entry` rule needs, so the same file cannot also be in a part for its
  layer. `layers` accepts only parts, so the layer order has to be written as one `mayNotImport`
  rule for each layer, each listing every layer above it.
- **#48.** A transitive rule follows `import()` like any other import. A bundler starts a new chunk
  at each `import()`, so "the initial bundle may not reach this package" cannot be written: the
  lazy path counts, and `imports: 'runtime'` does not change that.
- **#49.** Teams mark files with docblock tags, such as `/** @side-effect-free */`, that a bundler
  acts on. A rule over the tagged files would check that the tag is true, but no selector selects
  files by a tag. `directive` matches only directives such as `'use client'`.

## What changes

- **Layers of groups.** The `order` of a `layers` rule may name groups as well as parts. The rule
  means the same sentences: each layer may not import an earlier layer.
- **Static imports.** `imports` takes a fourth value, `'static'`: the runtime imports other than
  `import()`. In a transitive rule, a chain then stops at each `import()`.
- **Tags.** A selector `{ tag: '@side-effect-free' }` selects the files whose leading comments
  carry that JSDoc tag.

## Capabilities

### Modified capabilities

- `architecture-rules`: `layers` takes groups; `imports` takes `'static'`.
- `selectors`: a `tag` selector.

## Impact

- `archangel_config`: the `layers` order resolves group names; `ImportKinds::Static`; `Term::Tag`.
- `archangel_graph`: `FileFacts::tags`, read from the comments before the first statement.
- `archangel_rules`: `covers` reads an import's form as well as its kind; a layer message for
  groups; the `tag` term.
- `npm/archangel`: the types and the JSON schema.
- Docs: rule options, import options, layers, selectors.
