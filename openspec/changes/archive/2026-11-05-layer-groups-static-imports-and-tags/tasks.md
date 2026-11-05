# Tasks

- [x] Write tests: a layers rule of groups refuses an import toward an earlier group and permits the rest; its message names the layers; `isolate` and `entry` still refuse a group.
- [x] Let `layers` take groups.
- [x] Write tests: `imports: 'static'` skips `import()` and keeps `require()`; a transitive rule with it stops at `import()` and still reports a static chain.
- [x] Add `'static'` to `imports`.
- [x] Write tests: `{ tag }` selects a file whose docblock carries the tag, after a license comment too; not a tag later in the file; a value without `@` is refused.
- [x] Add the `tag` selector.
- [x] Update the types, the schema and the docs.
