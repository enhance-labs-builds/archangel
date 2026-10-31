# Security

Archangel reads a repository's source files and runs its TypeScript or JavaScript config with Node,
as a build tool does. It sends nothing over the network.

## Reporting a vulnerability

Report it privately with GitHub's
[private vulnerability reporting](https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing-information-about-vulnerabilities/privately-reporting-a-security-vulnerability)
on this repository, not in a public issue. Say what an attacker controls, what they gain, and how
to reproduce it. You get an answer within a week, and a fix is released before the report is made
public.

## Supported versions

Only the latest release gets fixes.

## What is in scope

- A repository's files making archangel write outside the repository, run code other than the
  config, or report a clean check for code that breaks a rule.
- The published binaries and npm packages not matching the source they were built from. Each npm
  package carries provenance from the release workflow, and each binary is in the release's
  `SHA256SUMS`.

A config is code you chose to run, so a config that does harm is out of scope, as a malicious
`package.json` script is for npm.
