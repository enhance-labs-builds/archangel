# Platform packages

Each folder is one npm package that holds the archangel binary for one platform, in `bin/`. The
release workflow builds each binary for the Rust target its `description` names, copies it into
`bin/`, and publishes the package; npm installs only the one whose `os`, `cpu` and `libc` match the
machine. The folders hold no binary in the repository.
