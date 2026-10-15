---
name: add-rule
description: Add a new command-rewrite rule to tipun safely, with its precondition checklist, unit tests and differential tests. Use when the user wants tipun to suggest a new transformation, such as "add a rule for X | Y".
---

# Add rule

A rule turns a parsed `Pipeline` into a better `Pipeline`, or returns `None`. A wrong suggestion
destroys trust, so every rule must prove its preconditions. Read the "Rule safety" section of
`CLAUDE.md` first.

If `src/rules/` does not exist yet, stop and tell the user the parser and rules milestones come
first.

## 1. Decide whether the rule is safe

Before writing code, write down for the user:
- The original form and the suggested form, with two or three concrete examples.
- Every way the two could differ: stdin vs file input, filenames in output, multiple files, flags
  that change meaning, flags that take an argument, `-` and `--`, output padding, exit status,
  BSD vs GNU behavior.
- The resulting precondition list. Anything not on the allow-list makes the rule return `None`.

If behavior cannot be preserved for the common case, recommend not adding the rule and stop. Do
not suggest third-party tools in v0.1.

## 2. Write the tests first

In the rule's `#[cfg(test)] mod tests`, add table-driven cases:
- positive: plain case, quoted file names, flags that are allowed, rest of the pipeline kept
- negative: one case per precondition, each showing that breaking it keeps tipun silent
- edge: `-`, `--`, file names starting with `-`, empty pattern, no spaces around `|`
- intentionally silent: look-alike commands where a rewrite would be wrong

Also check that the suggestion parses again and that the rule returns `None` on its own output.

## 3. Implement

- One file in `src/rules/`, one plain function `fn check(pipeline: &Pipeline) -> Option<Rewrite>`.
- A doc comment on the function listing the preconditions as a checklist.
- Command knowledge (allowed flags) lives in the rule file. Reuse the shared argument helper if it
  exists; do not add a global capability table.
- Build the new pipeline from the original words' `raw` text so quoting is preserved; only
  inserted words (like `-c`) are new.
- Register the rule in the `RULES` list, ordered by priority.
- Write a short, plain explanation string for the tip (no emoji).

## 4. Differential test

Add the positive cases to the equivalence tests in `tests/`: run the original and the suggestion
with `/bin/sh -c` on the fixtures and compare stdout and exit status. Add at least one "proof of
exclusion" case showing an excluded variant really does produce different output.

## 5. Finish

Run the `check` skill. Report the rule's preconditions, the documented exceptions (if any) and
the test counts. Do not commit unless the user asks.
