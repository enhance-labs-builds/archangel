# Design: Remove private material, fix wrong messages, and make the docs easy for agents

## In short

The public repository held the names and paths of a private monorepo. Some messages of 0.3.1 were
wrong or unclear. Agents could read the docs only as HTML pages or as one large file.

After this change, the private names are gone from every file, the messages say what their rules
refuse, and each page of the site has a Markdown file that `llms.txt` lists.

**How it works:**

1. A rename table replaces each private name with a neutral name of the same length, for example a
   package `ledger-document`. A name of the same length keeps each column that a test asserts.
2. The message of a violation picks its words from the sentence: an `entry` sentence gives the
   entry message, and the limits of the sentence follow its other side.
3. The check of empty parts counts a file that is in two parts for both parts.
4. starlight-llms-txt writes one Markdown file for each section and each page. One module of the
   site names these files, and the config, the head of each page and the link under its title read
   it.

### Decisions that need you

1. **History.** Older commits of the public repository still hold the private names. A rewrite of
   `main` and a force push removes them, and it changes the hash of each commit. This change does
   not do it.

## Decisions

**1. Names of the same length.** The tests assert columns of carets and messages. A neutral name
of the same length changes no number in a test, so the test of each behaviour stays as it was.

**2. The limits in a message come from the words of `archangel rules`.** `rules` already says
"through a relative specifier". The message uses the same words, so the two outputs agree.

**3. The plugin writes the Markdown.** starlight-llms-txt already renders a page to Markdown, with
the fixtures and the command outputs, for `llms-full.txt`. Its `customSets` option writes one file
for each set, so a set for each page needs no code of ours to render pages.

**4. Release 0.3.2.** The text of `archangel docs` is in the binary, and the site shows the text of
the published release. Only a release removes the private paths from those pages.

## Cost

No change to the work of a check. The message of a violation adds the words of its limits. The
check of empty parts reads the list of overlaps one time.

The build of the site renders each page three times more: for its own file, for its section and for
`llms-full.txt`. The build takes a few seconds more.

## Risks

- **A name in the archived designs still points at the private monorepo by its size.** The text says
  "a private 9,800-file reference monorepo". The README already gives the same size for the
  benchmark, so this says nothing new.
- **The plugin names a file by the slug of its label.** The site computes the same slug with the
  same library, and a test checks that each named file exists.

## Compatibility and release

The text of some messages changes, and `explain` adds a field to its JSON. Release 0.3.2. A second
pull request moves the site to 0.3.2 after the release.

## Limits

None found. The paths searched: the message of each condition in
`crates/archangel_rules/src/messages.rs`, the check of empty parts in
`crates/archangel_rules/src/check.rs`, and the explain command.

## Where in the code

| Part | Where |
|---|---|
| Messages | `crates/archangel_rules/src/messages.rs`, `crates/archangel_rules/src/words.rs` (`import_limits`) |
| Empty parts | `crates/archangel_rules/src/check.rs` (`empty_elements`) |
| Explain | `crates/archangel_rules/src/architecture.rs` (`SentenceAbout`, `sentence_roles`), `crates/archangel/src/commands/explain.rs` |
| Unknown command | `crates/archangel/src/commands.rs` (`usage_error`) |
| Markdown for agents | `site/src/agent-pages.mjs`, `site/src/route-data.ts`, `site/src/components/PageTitle.astro`, `site/astro.config.mjs` |
