# Changelog

Every release, newest first. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and archangel follows [semantic versioning](https://semver.org/) for its command line, its config,
its JSON output (`schemaVersion`) and its language server. The library crates have no stability
guarantee.

## Unreleased

The first release.

### Added

- Six rule kinds: `layers`, `isolate`, `entry`, `only`, `forbid` and `noCycles`, each with a name, a
  reason, a link, the kinds of import it checks, and examples it must accept and reject.
- A config in TypeScript, with `defineConfig` types that reject an undeclared element, or in JSON,
  with a published JSON schema.
- `archangel check`, `test`, `explain`, `why`, `watch`, `baseline`, `init`, `docs`, `completions`
  and `lsp`.
- Output as text, JSON, GitHub Actions annotations and SARIF.
- Exceptions in comments, each with a reason, and a baseline file for existing violations.
- A checked fix for each entry violation, offered on the command line and as a quick fix.
- An npm package with a binary for macOS, Linux (glibc and musl) and Windows, and a VS Code
  extension.
