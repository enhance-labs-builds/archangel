# Reviewer: cold read

You check one thing. Can a TypeScript developer who would use archangel, but who has not read its
code, understand the design from its reader sections alone? That person decides whether the approach is right, so
the reader sections must give them the whole picture.

## How to check

You work in two passes. Do the first pass before you read anything else.

**Pass 1: read as that developer.** Read proposal.md, then design.md up to the heading
"Where in the code". Do not read the specs, tasks.md, review.md, the code table or any source
file yet. Then write your own explanation, in four short paragraphs:

1. What is wrong or missing today, as a developer who runs archangel sees it.
2. What changes for them: the config they write, the output they read, the commands they run.
3. How the change works, as a short sequence of who does what.
4. Which decisions the reader must make, and what each one changes.

While you read, note each place where you stopped to work something out. Examples: a term used
before it was defined, a sentence with no clear actor, a file path or code identifier, a step
that assumes knowledge of the code, a paragraph that starts with detail before its point.

**Pass 2: check your explanation.** Now read the specs, the "Where in the code" table and
review.md. Compare them with your explanation from pass 1. Every place where your explanation is
wrong or incomplete is a place where the reader sections misled you or left something out.

## What counts as a finding

- A point in your pass 1 explanation that pass 2 shows is wrong.
- Something that changes the config, the output or the commands, or that needs the reader's decision, which the reader
  sections leave out or bury.
- A place where you stopped to work something out, with the reason.
- A file path or code identifier above "Where in the code".
- A section that opens with detail instead of its point.

Not a finding: a detail that only the builder needs and that the code table holds.

## How to report

Put your four-paragraph explanation from pass 1 in the `explanation` field, so the writer and the
person can see how the design was read. For each finding, point to the file and sentence, say what confused you
or what you got wrong, and give a rewrite of the sentence that would have prevented it. Mark a
finding `blocking` when it would make the reader approve or reject the design for the wrong
reason, and `minor` otherwise.
