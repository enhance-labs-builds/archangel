# Exceptions and baselines

Archangel has two ways to let code break a rule for now: an **exception** comment on one import,
and the **baseline** file for every violation that existed when a rule started. Both are counted in
every check's summary, per rule, so a team can watch the numbers go down. Neither is a way to turn
a rule off.

## Use an exception for one import you mean to keep

An exception is a comment directly above the import, with the rule's name and the reason:

```ts
// archangel-allow entry(entity): the migration reads the raw store before the entity exists
import { rawStore } from '@/entities/canvas/model/store';
```

- It permits that one import to break that one rule. The import is still checked against every
  other rule.
- It must give a reason after the colon. Without one it permits nothing and is an
  `exception-without-reason` diagnostic.
- It must name a rule the config has. A typo is an `exception-unknown-rule` diagnostic that lists
  the rule names.
- When the import no longer breaks the rule, the exception is an `exception-unused` diagnostic, so
  it goes away when the code is fixed.
- Comments on consecutive lines apply together to the import after them, so one import can have an
  exception for two rules. A blank line between the comment and the import breaks the link.

Use an exception when the import is a decision someone made and wrote down: a migration, a
generated file, an adapter that has to reach inside a module. The reason is for the next reader.

## Use the baseline when a rule starts on code that already breaks it

```sh
archangel baseline add
```

writes every current violation to `archangel.baseline.json` beside the config. A check then fails
only for violations that are not in the baseline.

- An entry records the rule, the importing file, the specifier and a count, not a line, so an edit
  above the import does not break the match. A cycle's entry records every file of its group.
- The file is sorted, so two runs on the same code write the same bytes and the diff of a pull
  request shows exactly what changed.
- A check never writes the baseline. When the code fixes a violation, the entry is a
  `baseline-stale` diagnostic that names `archangel baseline prune`, which lowers or removes
  entries and never adds one or raises a count. So the baseline can only shrink.
- A baseline entry for a rule the config does not have is `baseline-stale` too, and a baseline that
  does not parse stops the check with `baseline-invalid` and its line and column.

Use the baseline to turn a rule on today without fixing the whole repository first. Commit the
file, and let each pull request that touches old code prune what it fixed.

## Which one

| You have | Use |
| --- | --- |
| One import that breaks a rule on purpose, with a reason worth reading | An exception |
| Many violations from before the rule existed | The baseline |
| A rule that is wrong for part of the code | Change the rule or the elements, not either of these |
| Files that should not be checked at all, such as generated code | The config's `ignore` list |
