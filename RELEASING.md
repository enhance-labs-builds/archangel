# Releasing

A release publishes six crates to crates.io and eight packages to npm from one version tag. The
release workflow, `.github/workflows/release.yml`, does the publishing. Nobody publishes from a
laptop.

## What the registries trust

Both registries use trusted publishing: they trust this repository's release workflow, and no
registry token is stored anywhere. Each package's trusted publisher is set to:

| Field | Value |
| --- | --- |
| Repository | `enhance-labs-builds/archangel` |
| Workflow file | `release.yml` |
| Environment | `release` |

The `release` environment accepts only tags that match `v*.*.*`, and a maintainer approves each
run.

## Releasing a version

1. **Set the version everywhere, in one pull request.** That covers `version` in the root
   `Cargo.toml` and the `archangel_*` dependency versions there, every `package.json` under `npm/`
   with the `optionalDependencies` of `npm/archangel/package.json`, and
   `editors/vscode/package.json`. `scripts/check` runs `scripts/release-version.mjs`, which lists
   every place that disagrees.
2. **Rehearse.** Run the release workflow by hand from `main`. A run started by hand never publishes.
   It builds every binary, compares the checksums, and runs both registries' dry runs.
3. **Tag the merged commit and push the tag:**

   ```sh
   git tag v0.1.0
   git push origin v0.1.0
   ```

   The first job refuses a tag that is not the version in the files.
4. **Approve the run** when it waits on the `release` environment. It then:
   - publishes the crates, then the npm packages with provenance;
   - installs both from the registries on every platform;
   - checks the provenance signatures;
   - creates the GitHub release with the checksums.

A published version is permanent. If a release is wrong, yank the crate version, deprecate the npm
version, and release the next patch version.
