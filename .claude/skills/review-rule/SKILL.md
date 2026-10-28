---
name: review-rule
description: Adversarially review an existing tipun rule for inputs where its suggestion would behave differently from the original command, and turn each finding into a failing test. Use before merging a rule change, when a rule seems suspicious, or when the user asks to review or audit a rule.
---

# Review rule

Assume the rule is wrong and try to prove it. A single wrong suggestion breaks the core promise in
`CLAUDE.md`: silence over a questionable suggestion.

## Steps

1. Read the rule file in `src/rules/`, its preconditions (the early `return None` checks), its
   tests, and the shared helpers it uses (argument parsing, rendering). If there are no rules yet,
   say so and stop.
2. List the rule's claim: "for inputs matching P, rewrite O into S".
3. Attack it. For each category, try to build an input that passes the preconditions but where O
   and S differ in stdout, file side effects or exit status:
   - flags: ones that change meaning (`-o`, `-l`, `-r`, `-f`, `-n`, `-k`), combined (`-in`),
     attached arguments (`-n5`), long options, `--`, unknown flags
   - operands: multiple files, `-`, names starting with `-`, an operand that is really an
     option's argument, an output-file operand (`uniq A B`)
   - stdin vs file: filename in the output, "(standard input)" labels, seekable input, binary
     files
   - redirections: on which stage, `2>&1` placement, input redirects that override the pipe,
     zsh MULTIOS (`> a > b`)
   - quoting: quoted names with spaces, empty strings, quotes containing `|` or `>`
   - pipeline position: matched stages in the middle, extra stages before or after
   - environment: aliases and prefixes (`sudo`, `time`, `command`), BSD vs GNU tools, locale
   - output format: padding, trailing newline, exit status on zero matches
4. Verify each candidate rather than guessing: run `tipun suggest` on it, and run the original and
   the suggestion with `/bin/sh -c` on small files in a temp directory. Use the `equivalence`
   skill if it helps.

## Report

For each confirmed problem:
- the input, the suggestion, and the observed difference (exact output)
- the precondition that is missing or too loose
- a failing unit test case to add, and the minimal fix

Also list candidates that turned out safe, in one line each, so the user knows they were checked.
If nothing was found, say so plainly; do not invent issues.

Ask before changing code. If the user agrees, add the failing tests first, then fix the rule, then
run the `check` skill.
