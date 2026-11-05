# Tasks

- [x] Write tests: a namespace and a dynamic import of a subpath that does not export a banned name pass; one whose module re-exports it through a relative `export *` breaks the rule; a CommonJS module and a built-in keep every name; one subpath is parsed once.
- [x] Find a package module's names in `archangel_graph`, keep them on the import, and use them in `names`.
- [x] Write tests: an exception above a multi-line import permits it; one above the line before an import permits nothing.
- [x] Let an exception above a statement apply to its import.
- [x] Write tests: an `allowUnresolved` entry for a missing tsconfig base gives no diagnostic and is counted; without it, the diagnostic names the entry to add.
- [x] Permit a missing tsconfig base through `allowUnresolved`.
- [x] Update the docs and the diagnostic pages.
- [x] Set every version to 0.3.4 and add the changelog entry.
