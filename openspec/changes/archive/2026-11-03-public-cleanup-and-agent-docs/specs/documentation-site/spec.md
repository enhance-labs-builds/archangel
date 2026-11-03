# Spec Delta

## ADDED Requirements

### Requirement: A coding agent reads each page as Markdown

Each page of the site MUST have a Markdown file, which the build writes from the same page. The head of the page MUST name the file in a link of the type `text/markdown`, and the page MUST show a link to it. `llms.txt` MUST list the file with the description of the page.

#### Scenario: The built site

- **GIVEN** the built site
- **WHEN** the test reads each page
- **THEN** each page names a Markdown file that exists, shows a link to it, and `llms.txt` lists it, and the file has the title of the page
- **TEST** `site/test/agents.test.mjs` › "each page links to its Markdown, and llms.txt lists it"
