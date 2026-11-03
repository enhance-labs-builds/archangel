# Tasks

- [ ] Write the tests: `site/test/examples.test.mjs`, `site/test/pages.test.mjs` and
      `site/test/diagnostics.test.mjs` (integration, with the published package), one test for each
      scenario, named after the scenario.
- [ ] Add the Astro Starlight site in `site/`, with the theme, the home page and the sidebar.
- [ ] Write the fixtures in `site/examples/`, and their expected output.
- [ ] Write the pages: Start, Guides, Concepts and Reference.
- [ ] Generate a page for each diagnostic code from `archangel docs`.
- [ ] Add the site to `scripts/check` and to the `Checks` job, and add `.github/workflows/site.yml`.
- [ ] Link the site from the README.
- [ ] Run `scripts/check` locally.
