---
name: check
description: Run tipun's pre-commit checklist (tests, warning-free build, clippy with -D warnings, rustfmt check) and report what fails. Use before committing, after finishing a step, or when the user asks to check, verify or lint the project.
---

# Check

Run the pre-commit checklist from `CLAUDE.md` and report the results. Do not fix anything unless
the user asks.

## Steps

Run from the repository root. If `cargo` is not on `PATH`, use `~/.cargo/bin/cargo`.

1. `cargo fmt --check`
2. `cargo build 2>&1` and look for any `warning:` lines
3. `cargo clippy --all-targets -- -D warnings`
4. `cargo test`
5. Search the diff (`git diff` and untracked `.rs` files) for `dbg!`, `todo!`, `unimplemented!`,
   leftover `println!` debugging and commented-out code.

Run every step even if an earlier one fails, so the report is complete.

## Report

One line per step: pass or fail. For each failure, show the relevant error lines (not the full
output) with `file:line`, and say in one sentence what is wrong. End with the overall verdict:
ready to commit, or the list of things to fix.

If `cargo fmt --check` is the only failure, offer to run `cargo fmt`.
