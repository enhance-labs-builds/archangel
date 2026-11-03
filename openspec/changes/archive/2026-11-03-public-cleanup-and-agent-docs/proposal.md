# Remove private material, fix wrong messages, and make the docs easy for agents

## Why

The public repository holds the names, paths and rule names of a private product monorepo. They
are in a docs example, in tests, in specs, in archived designs and in the text that
`archangel docs` prints. They must not be public.

Writing the documentation site found messages in 0.3.1 that are wrong or unclear, and two places
where `docs/` says something that archangel does not do. A developer, or a coding agent, acts on
these messages, so a wrong message gives a wrong fix.

The owner also asked that coding agents can read the docs easily. Archangel is made for agents,
but today an agent must read whole HTML pages, or one 370 KB text file.

## What changes

- **Private material.** Each private name is replaced by a neutral name of the same length, so
  that no column in a test moves. The docs example of a monorepo becomes `monorepo-slice`. The
  archived designs call the private monorepo "the reference monorepo".
- **Messages.**
  - A rule of several sentences with an `entry` sentence gives the entry message, not "may not
    import" with an empty list.
  - A `mayNotImport` message names the limits of its rule: the instance, the names, the form of
    the specifier.
  - A package pattern reads as words: `*` is "any package", and `@scope/*` is "a package in
    `@scope`".
  - A part that matches only files that another part also matches is not `element-empty`.
  - The list of commands after an unknown command is aligned, and it does not show hidden commands.
  - `archangel explain <file>` says which rules judge the file's own imports and which judge
    imports of the file.
  - The text of `archangel docs` and the help say "part" and "group", the words of the config.
- **Docs.** `docs/rules.md` says what `within` does on `mayOnlyBeImportedBy`, and its folder
  example passes on a repository without the folder.
- **Docs for agents.**
  - Each page of the site has a Markdown file, and each section has one.
  - `llms.txt` lists each of them with its description.
  - Each page links to its Markdown file in its head, and with a visible link.
  - A new guide tells a developer how to give the docs to an agent.
- **Release 0.3.2**, so that the published text of `archangel docs` has no private names. A
  second pull request moves the site to 0.3.2 after the release is published.

## Capabilities

### Modified capabilities

- `diagnostics`: the messages above.
- `command-line`: the list of commands, and `explain`.
- `documentation-site`: Markdown for agents.

## Out of scope

- The history of the public repository still holds the private names in older commits. Removing
  them needs a rewrite of `main` and a force push. The owner decides that separately.

## Impact

- `crates/archangel_rules`: messages, the empty-part check, the docs text.
- `crates/archangel`: the unknown-command list, `explain`, the help text.
- `site/`: Markdown for agents, a new guide, the pinned version after the release.
- No change to the config format. The text of some messages changes, so 0.3.2 is a patch release.
