---
name: next-step
description: Find the next small step in tipun's v0.1 roadmap, explain it, and implement only that step. Use when the user says "next step", "continue" or asks what to build next.
---

# Next step

The user is new to Rust and wants to build tipun in small, understandable steps.

## Roadmap (v0.1)

1. CLI skeleton with `suggest` (exit 1 = silent).
2. Lexer: words with `raw`/`value`, quotes, pipes; reject everything else.
3. Lexer: redirections `>`, `>>`, `<`, `n>`, `n>&m`.
4. Parser: tokens into `Pipeline { stages: Vec<Command { name, args, redirects }> }`.
5. Render: pipeline back to text from `raw`; round-trip tests.
6. Argument helper: split a command's args into flags and operands against a per-command spec.
7. Rule `useless-cat` for grep, head, tail, sort, uniq (append the file).
8. Rule `useless-cat` for wc, tr (`< FILE` instead).
9. Rules `sort-uniq` and `grep-count`, alias check, redirect-placement check.
10. State file and throttling, `analyze` subcommand, tip rendering.
11. zsh integration, `init zsh`, smoke test.
12. Benchmark, README, release v0.1.0.

## Steps

1. Look at `src/`, `Cargo.toml` and `git log --oneline` to work out which roadmap steps are done.
   Trust the code over this list.
2. Tell the user, in two or three sentences, what the next step is and why it comes next.
3. Implement only that step:
   - Add a file or module only if this step uses it.
   - Everything added must be reachable from `main` or a test in this step, so the build has no
     `never used` warnings. If a piece cannot be used yet, wire it into an existing command
     (for example `suggest`) or postpone it.
   - Follow `CLAUDE.md`: minimal comments, no unwrap outside tests, no new dependencies without
     asking.
   - Add table-driven tests, including inputs that must be rejected.
4. Run the `check` skill.
5. Summarize what changed and explain any Rust concept the user may not know (ownership,
   `Option`/`Result`, `match`, traits, lifetimes) in the chat, briefly.
6. Name the following step and stop. Do not start it, and do not commit unless asked.
