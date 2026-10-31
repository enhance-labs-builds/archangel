# Contributing

Archangel is designed before it is built. A change starts as an OpenSpec change in
`openspec/changes/`: a short proposal, the behaviour as requirements with test scenarios, a design
a TypeScript developer can read without knowing the code, and a task list. The design is
reviewed, then built test-first in the same pull request, then archived into `openspec/specs/`.

## Before you start

- For a small fix, open a pull request with the fix and a test that fails without it.
- For anything else, open an issue that says what you need and why. That issue is the brief the
  design answers.

## Working on the code

To build archangel and run every test, you need only Rust and Node 24, which runs the TypeScript
configs in the tests:

```bash
cargo test --workspace
```

Before you open a pull request, `scripts/check` runs everything CI runs. For that you also need
the checkers below; `rust-toolchain.toml` pins Rust 1.93, and CI pins the same versions.

```bash
brew install actionlint zizmor typos-cli lychee cargo-deny
npm ci --ignore-scripts
```

Install [zanity](https://github.com/benomahony/zanity) 0.1.33 from its release page, and check the
download's SHA-256 against the one the release lists.

`scripts/check` runs every check: formatting, clippy with every lint at `deny`, the tests,
dependency licences and advisories, zanity, spelling, the workflows, Markdown links and the
OpenSpec changes. CI runs the same script, so run it before you push:

```bash
scripts/check
```

Issues are the next thing to build, not a backlog. An issue nobody is about to build is closed
with a note saying so; if it matters, it will come back, and it is welcome to.

## Working with a coding agent

Run the tests in watch mode beside the agent, so the agent sees a failure while it is still
working rather than after its turn ends. [overwatch](https://github.com/benomahony/overwatch) does
this for Claude Code, Codex, OpenCode, Pi and Neovim, and finds `cargo test` without any config.


[AGENTS.md](AGENTS.md) holds the rules every contributor follows, human or agent. With Claude
Code, `/design <issue>` writes and reviews a design as a draft pull request, and
`/implement <pull request>` builds it. The skills in `.claude/skills/` say how.
