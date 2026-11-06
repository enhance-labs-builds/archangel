# diagnostics

## ADDED Requirements

### Requirement: The entry fix reaches an entry file in a sibling folder

The fix for an entry violation MUST also try the specifiers that keep the import's leading names and lead from the folder they name to an entry file, without its extension and a last `index`, shortest first, under the checks that every fix passes. A template import MUST get no fix, and its help MUST say why.

#### Scenario: A public folder

- **GIVEN** a pack whose entry files are `public/**`, `public/index.ts` that re-exports `Button`, and `import { Button } from '@/shared/ui/components/Button'`
- **WHEN** archangel makes a check
- **THEN** the entry violation's fix is `@/shared/ui/public`
- **TEST** `crates/archangel_engine/tests/fixes.rs` › "entry_fix_reaches_an_entry_file_in_a_sibling_folder"

#### Scenario: A template import

- **GIVEN** an entry violation of a template import
- **WHEN** archangel makes a check
- **THEN** it has no fix, and its help says that one specifier cannot replace a template
- **TEST** `crates/archangel_engine/tests/fixes.rs` › "entry_fix_is_not_offered_for_a_template_import"
