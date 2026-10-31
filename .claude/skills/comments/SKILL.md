---
name: comments
description: Decide whether a code comment earns its place. Use when writing, editing or reviewing a comment or doc comment, or when editing a comment marked @justified.
---

# Comments

A comment is the only thing in this repository that no compiler checks and no test can fail. It
can silently become a lie, so it has to earn its place. These rules are Robert Martin's taxonomy
from *Clean Code* (ch. 4) and John Ousterhout's rule from *A Philosophy of Software Design*
(ch. 12–16).

## The one test

A comment must sit at a different level of detail from the code. A comment at the same level
repeats it: Ousterhout's red flag "Comment Repeats Code".

Before writing one, ask: is it **lower-level** than the code, a precise fact the code cannot
state, such as a unit, a range or a protocol quirk? Or **higher-level**, saying why this exists or
what it is for? If neither, delete it and rename something instead.

## The six justifications

| Category | When |
| --- | --- |
| `intent` | Why the code does this, where the reason is not visible locally. |
| `clarification` | Translating something obscure you cannot change, such as a crate's return value or a protocol field. |
| `warning` | A consequence the next editor will not expect. |
| `amplification` | Something that looks incidental and is load-bearing. |
| `informative` | A fact the code cannot carry, such as the shape a pattern matches or a unit. |
| `legal` | Licence and copyright. |

There is no `todo`. This repository carries no debt: deferred work is done now, or it is not
done. A TODO comment is a promise nobody tracks, so it is not a justification.

## What does not earn its place

- **Journal comments** narrating how the code got here. Git holds history.
- **Redundant comments** restating the line below, and **noise** such as `// the constructor`.
- **Position markers**: banners and `// ===== SECTION =====`.
- **Commented-out code.** Delete it. It is in Git.
- **Attributions and bylines**: "added by", author tags.
- **Misleading comments**: anything no longer true. The most expensive kind, because it is trusted.
- **Mandated comments**: a doc comment on every function whether or not it has something to say.
- **Nonlocal information**: a decision that lives in another file. It rots when that file moves.
  So do not explain another tool's internals, and do not invent vocabulary where an established
  term exists.

A doc comment on a public item describes the contract, not the mechanism. How it works belongs
inside the body, or nowhere: Ousterhout's red flag "Implementation Documentation Contaminates
Interface". Public items in a published crate are the exception that needs doc comments, because
docs.rs is where their users read the contract.

## Keeping one

When a comment earns more than a line or two, open it with its category:

```rust
// @justified warning
// The resolver caches every lookup, including misses. A file created after the first check is
// invisible to it until the cache for that directory is cleared.
```

`@justified` takes one of the six categories and nothing else. When you edit a comment that
carries one, you are editing something already argued for. Apply these rules before adding to it,
so a justified block does not slowly collect unjustified paragraphs.
