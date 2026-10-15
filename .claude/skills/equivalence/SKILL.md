---
name: equivalence
description: Check whether a tipun suggestion behaves exactly like the original command by running both in sh and zsh on edge-case files and comparing output and exit codes. Use when the user wants to verify a suggestion by hand, doubts a rewrite, or asks "is X the same as Y".
---

# Equivalence

Run an original command and its suggested rewrite side by side and report any difference.

## Steps

1. Get the pair:
   - If the user gave both commands, use them.
   - If they gave only the original, get the suggestion with `cargo run -q -- suggest '<original>'`
     (fall back to `~/.cargo/bin/cargo`). If tipun stays silent, say so and stop.
2. Make a fresh temp directory under the scratchpad (or `mktemp -d`) and create fixture files.
   Use the file names from the commands, and create each of these variants for the main input
   file, one at a time:
   - normal text with a few matching and non-matching lines
   - empty file
   - no trailing newline
   - duplicate lines, case variants (`a`, `A`), numeric variants (`1`, `01`)
   - CRLF line endings
   - UTF-8 text
   - a name with spaces, if the command quotes one
3. For each fixture and each shell (`/bin/sh -c` and `zsh -f -c`), run both commands inside the
   temp directory and capture stdout, stderr and the exit code separately. Do not let the
   commands touch files outside the temp directory; refuse commands that delete or modify files
   unless the user confirms.
4. Compare stdout byte for byte and compare exit codes. Stderr differences are allowed by the
   equivalence contract in `CLAUDE.md`, but report them anyway.

## Report

A table: fixture, shell, stdout same?, exit same?, notes. Show the actual differing output for
any mismatch (use `diff` or `od -c` when whitespace matters).

End with a verdict:
- equivalent on all fixtures
- differs only in a documented exception (name it)
- not equivalent: the smallest failing case, and which rule precondition should exclude it

Mention that the results reflect this machine's tools (BSD on macOS, GNU on Linux); the other
platform can differ.
