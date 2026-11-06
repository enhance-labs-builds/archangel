# Design

## Decisions that need you

1. **#36: only type imports fall back to `@types`.** A runtime import of a package that is not
   installed fails when the code runs, so it stays unresolved.
2. **#37: `!` patterns, not `not`.** A part is a list of patterns, and `!` is how `.gitignore` and
   most glob lists leave files out. `part-overlap` now says so. A list of only `!` patterns is
   refused.
3. **#41: `./`, not guessing.** A bare `app.config.ts` stays a name, because a name that looks like
   a file is still a name. The error suggests `./app.config.ts`, which now works.
4. **#44: JSON and SARIF stops go to standard output; text and GitHub stops stay on standard
   error.** A machine reads one stream; a person and the Actions runner read the other.
5. **#34: a transitive rule refuses a subpath.** A chain ends at a package, so a subpath could
   never end one; the rule is refused rather than silent.
6. **#42: `severity` on every rule, `--max-warnings` on `check`.** A warning is reported in every
   format, is counted, and passes; `--max-warnings 0` fails on any. The baseline and exceptions
   treat warnings like any violation.
7. **#43: no rename.** When the entry exports a name under another name, there is no fix, and the
   help says what the entry is missing.
8. **#46: Vite's rules, one violation per file.** A template import must start with `./` or `../`,
   end in a written extension, and, in its own folder, write part of the file name. Each `${…}`
   is one file or folder name. Each file it reaches is judged, and each violation names its file.
   A template that matches no file is `import-unresolved`. No fix is offered for one.

## How it works

- **#36:** `resolve_type_import` tries `@types/<name>`, or `@types/<scope>__<name>`, after a
  `NotFound`, and names the package as the import does.
- **#37:** `Element::not`; the matcher skips a part whose `!` pattern matches the path.
- **#40:** only when a set is empty and the config has ignore lines, the graph walks again without
  them and names the first line that removes a file the set would hold.
- **#34:** `package_matches` compares a subpath pattern with the import's module (`name/subpath`),
  and any other pattern with the package. Classes are kept per module.
- **#42:** `Rule::warns`; the run marks each violation of such a rule.
- **#46:** the parser keeps the template's pattern on the import. Resolution matches it against
  the walked files and stands one import in for each file. A file appearing or disappearing
  already resolves every import again; a file that is not source does too, when a template import
  exists.

**Cost.** Nothing on the check path for a config without the new forms, except one comparison per
evaluated import for `imports` and one per package pattern for subpaths. A template import costs
one pass over the file list; an empty set with ignore lines costs one more walk.
