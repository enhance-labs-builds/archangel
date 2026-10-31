# Glossary

Every spec in `openspec/specs` uses the terms below with exactly the meanings below.

A term in this list has one meaning only. When the code uses a different word for the same thing,
the specs use the term here. The words after "Not:" are words that the specs must not use for
that meaning. A change adds each new term here before a spec uses it.

## Terms

### Check

One run of archangel over a repository with a config. A check reads the repository, applies every
rule, and gives the results. The command line, watch mode and the language server all make checks.

Not: scan, analysis, lint run.


### Config

The file in which a developer declares the elements and the rules of a repository. It is
`archangel.config.ts`, `archangel.config.js`, `archangel.config.mjs` or `archangel.config.json`.

Not: configuration file, settings, rule set.


### Source file

A JavaScript or TypeScript file in the repository that archangel reads for its imports. Declaration
files are source files too.

Not: module (for a file), input file.


### Import

A place in a source file that names another file or package by a specifier. These are imports:
import statements, export-from statements, dynamic imports and require calls with a literal
specifier, TypeScript import-equals declarations with a require, import types such as
`import('./x').Foo`, and triple-slash reference directives.

Not: dependency, edge, reference.


### Specifier

The text in an import that names what is imported, for example `./store` or `react`.

Not: path (for this text), module name, request.


### Type import

An import whose statement has the `type` modifier, or in which every imported name has the `type`
modifier. An import type, a triple-slash reference and every import in a declaration file are type
imports too.

Not: type-only dependency, erased import.


### Runtime import

An import that stays in the compiled JavaScript. Every import that is not a type import is a
runtime import.

Not: value import, real import.


### Resolution

The source file, other repository file or package that an import points to. An import whose
resolution archangel cannot find is an unresolved import.

Not: target, destination, resolved module.


### Pattern

A path, relative to the config folder or, for an entry file, to the instance root, in which `**`
matches any number of folders, `*` matches part of one folder or file name, braces with a comma list
alternatives, and a capture matches a part of one folder or file name that is not empty.

Not: glob (for an element pattern), matcher.


### Capture

A part of a pattern written as a name in braces, such as `{feature}`. It matches a part of one
folder or file name that is not empty, and the matched text is the capture value.

Not: placeholder, parameter, slot.


### Element

A named kind of file or package that the config declares, such as "feature" or "react". A file
element has patterns for repository files. A package element has a list of package names.

Not: layer (for one element), module, category, role.


### Element instance

One element together with one set of capture values. With the pattern `src/features/{feature}/**`,
each feature folder is a different instance of the element "feature".

Not: slice, sub-element.


### Instance root

The folder of an element instance: the part of a matched path before the first `*` or `**` of the
pattern, with each capture replaced by its value.

Not: base path, root folder.


### Entry file

A file through which code outside an element instance is permitted to import that instance. An
entry rule declares the entry files, relative to the instance root.

Not: barrel, public file, index (for this meaning).


### Rule

A statement in the config: one selector and one condition, written as a sentence or a shorthand kind,
with a name that is unique in the config.

Not: constraint, policy, check (for a rule).


### Violation

One import, one file, folder or instance, or one group of files that import each other in a cycle,
that a rule does not permit.

Not: error (for a violation), finding, problem, offence.


### Diagnostic

One result that archangel gives the developer: a violation, or a problem with the config, a source
file, an import, an exception or the baseline. Each diagnostic has a code.

Not: message, warning (as a general term), finding.


### Fix

A change to a source file that archangel offers for a diagnostic. Archangel offers a fix only
after it applies the fix in memory and the fix removes the diagnostic.

Not: suggestion, auto-fix, quick fix (except as the editor's name for it).


### Example

A small set of files, written in the config as file paths and contents, that a rule must accept or
must reject. An accepted example must not cause a violation of its rule. A rejected example must
cause one.

Not: fixture, test case, rail.


### Exception

A comment above one import that permits that import to break one named rule, and gives the reason.

Not: suppression, ignore comment, disable comment, waiver.


### Baseline

A file that records the violations which existed when a team started to use a rule, so that a check
fails only for violations that are not in it.

Not: snapshot, allowlist, suppressions file.


### Work count

A count of one kind of work that a check did, such as files parsed or imports resolved. Tests
assert work counts instead of durations.

Not: metric, statistic, performance counter.


### Rule builder

A function that the config's rules function receives: `rule` for a sentence, and `layers`,
`isolate` and `entry` for the shorthand kinds. Each builder returns one rule.

Not: helper, factory.


### Summary

The part of a check's output after the diagnostics. It counts the files and imports that the check
read, the exceptions and baseline matches for each rule, and everything that archangel did not
follow, such as computed specifiers and links to folders.

Not: report (for this part), footer, totals.


### Package manifest

A `package.json` file.

Not: package file, manifest (alone).


### Fault control

A build of archangel with one deliberate defect, such as a second parse of each file, that a
work-count test must reject. It proves that the test can fail.

Not: negative control, mutant (for this meaning).


### Selector

A named set of files, declared by patterns, elements, instances, directives or other selectors.
Selectors may overlap: a file is in every selector it matches, and in at most one element.

Not: tag, group, category, file set.


### Class

The files that are in the same element and exactly the same selectors, or the packages in the same
package element and the same inline package sides. Archangel checks an import by the pair of classes
at its two ends.

Not: bucket, partition, signature.


### Condition

What a rule asks of the files in its selector: one of the import, structure, export or no-cycles
conditions, or a plugin condition. Every rule has exactly one.

Not: predicate, constraint, check (a check is one run of archangel).


### Shorthand kind

One of the three rule kinds layers, isolate and entry, each of which stands for one or more
sentences.

Not: rule type, macro, legacy rule.


### Preset

A function that returns elements, selectors, rules and their examples, composed into a config with
`extends`.

Not: shareable config, ruleset, template.


### Plugin

A package whose `definePlugin` object provides plugin conditions; the same package may also export
presets as functions.

Not: extension, add-on.


### Plugin condition

A condition written in TypeScript in a plugin. It receives a view and reports violations.

Not: custom rule, plugin rule (a plugin rule is a rule that uses a plugin condition), visitor.


### View

The data that archangel gives a plugin condition: the records of the files its filter covers, with
their elements, captures, selectors and the facts the condition reads, and a schema version.

Not: graph dump, snapshot, AST.


### Plugin process

The Node process that loaded the config and runs the plugin conditions, for one command or for the
length of an editor session.

Not: sidecar, worker, daemon.


### Sentence

A rule written as one selector and one condition, with `rule()`. Each shorthand kind compiles to
sentences, and `archangel explain` shows rules as sentences.

Not: rule definition, expression, query.


### Filter

What a plugin condition is about: the rule's selector, and the facts the condition reads (imports,
importers or exports). Archangel evaluates the filter itself and sends only what it covers.

Not: matcher, scope, hook filter.


### Name

A file's name up to its first dot: `button.view.tsx` has the name `button`. Name styles and the
`{FileName}` spellings use it. A folder's name is the whole last part of its path.

Not: base name, stem.


### Full name

A file's name without its last extension: `button.view.tsx` has the full name `button.view`. Name
patterns match against it.

Not: file name, base name.
