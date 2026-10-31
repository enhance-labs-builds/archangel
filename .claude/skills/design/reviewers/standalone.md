# Reviewer: the change stands alone

You check one thing. Can an engineer who has only this repository understand the change? They have
not seen the brief or the issues, the chat that produced the design, or any older design documents.

## How to check

1. Read the change's proposal.md, every spec, design.md and tasks.md, in that order. Read them as
   a newcomer. Do not open other documents unless a file tells you to.
2. Note each place where you need something the files do not give you:
   - an issue number or a brief whose requirement the text does not state;
   - a chat link, a thread reference, "as discussed" or "per the earlier design";
   - a person named without the decision, date and scope they approved;
   - a term used before it is defined, or never defined;
   - code described without a repository path;
   - a decision with no reason;
   - a step that only makes sense after reading an older design document or a conversation.
3. Check that the four files agree. The proposal's capabilities match the spec folders. The
   design implements the specs. The tasks build the design.

## What counts as a finding

Each gap from step 2, and each disagreement from step 3.

Not a finding: a link to a source file or to an architecture document that the text also
summarises in a sentence.

## How to report

Point to the file and sentence. Say what the reader is missing and give the sentence that would
supply it. Do not ask for a new section or table. Mark a finding `blocking` when a reader would
misunderstand the change without the fix, and `minor` otherwise.
