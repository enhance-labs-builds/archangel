# Design

## Decisions that need you

1. **#47: only `layers` takes groups.** The issue also names `isolate` and `entry`, but both need a
   capture, so each instance is its own boundary. A group has no capture. Those two stay
   part-only, and their error still says so.
2. **#48: `'static'` is a fourth value of `imports`, not a `stopAt` option.** Which imports a rule
   follows is what `imports` already chooses. `require()` counts as static, because a bundler puts
   it in the chunk of the file that calls it.
3. **#49: a tag is read from every comment before the first statement**, line or block, so a
   license comment above the docblock does not hide it. A tag must start with `@`.

## 1. Layers of groups (#47)

`order` names parts, package parts or groups. The rule compiles to the sentences it always did, with
`Term::Selector` for a group, and `RuleKind::Layers` keeps the layer names.

- When every layer is a part, nothing changes: the same `considers` test and the same message.
- When a layer is a group, `considers` uses the sentences, as for any rule of sentences, and the
  message names the layers: `"featuresLayer" may not import a file of an earlier layer
  (pagesLayer), through "../pages/home"`, with the help `"featuresLayer" may import only
  "featuresLayer", "sharedLayer"`.
- Groups can overlap. A file in two layers is judged as a file of each, so an import that breaks
  both gives one violation for each layer.

**Cost.** None for a check of parts. A group layer adds one sentence, as a `mayNotImport` rule does.

## 2. Static imports (#48)

`ImportKinds::Static` covers an import whose kind is runtime and whose form is not `Dynamic`.
`covers` takes the import, not only its kind. A transitive rule follows only the imports that its
`imports` covers, so with `'static'` a chain stops at `import()`. `'static'` is accepted where
`'runtime'` is.

**Cost.** One more comparison for each import that a rule evaluates.

## 3. Tags (#49)

`FileFacts::tags` holds the tags of the comments before the first statement or directive: each
`@name` at the start of a comment line, after optional whitespace and `*`, that ends at whitespace or
the end of the line. `{ tag: '@x' }` selects the files whose tags include `@x`. A value without the
leading `@` is `config-invalid`, with the fix.

**Cost.** One pass over the leading comments of each parsed file, which the parser has already
found. A file that has no leading comments costs nothing.

## Where in the code

- `crates/archangel_config/src/rules.rs`: the `layers` order, the `imports` values.
- `crates/archangel_config/src/shorthand.rs`: `layer_sentences` with groups.
- `crates/archangel_config/src/model.rs`: `ImportKinds::Static`, `Term::Tag`, `RuleKind::Layers`.
- `crates/archangel_config/src/selectors.rs`: `{ tag }`.
- `crates/archangel_graph/src/imports.rs`: `FileFacts::tags`.
- `crates/archangel_rules/src/architecture.rs`, `messages.rs`, `classes.rs`, `words.rs`.
