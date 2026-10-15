---
name: try-command
description: Run `tipun suggest` on one or more shell commands and explain why tipun suggested something or stayed silent. Use when the user asks what tipun does with a command, wants to test a rule by hand, or asks why a command got no tip.
---

# Try command

Show how tipun handles the given commands, and why. Read-only: do not change code.

## Steps

1. Take the commands from the user's message. If none were given, ask for one.
2. For each command, run from the repository root (fall back to `~/.cargo/bin/cargo` if `cargo`
   is not on `PATH`), quoting the command with single quotes so the shell passes it unchanged:
   `cargo run -q -- suggest '<command>'; echo "exit: $?"`
   Exit 0 means a suggestion was printed; exit 1 means tipun stayed silent.
3. Explain the outcome by reading the code that decided it:
   - Silent because the lexer or parser rejected it: name the construct (for example `$VAR`, a
     glob, `&&`, a here-doc) and the place in `src/` that rejects it.
   - Silent because no rule matched: name the closest rule and the precondition that failed (an
     unexpected flag, more than one file, a redirect on the wrong stage, and so on).
   - Suggested: name the rule and confirm why the rewrite preserves behavior under the
     equivalence contract in `CLAUDE.md`.

If a subcommand or module the explanation needs does not exist yet, say so plainly instead of
guessing.

## Report

A short table: command, suggestion (or "silent"), reason. If a result looks wrong (a suggestion
that could change behavior, or silence on a case a rule is meant to cover), flag it clearly as a
possible bug.
