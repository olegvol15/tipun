# tipun

A local, deterministic, non-AI terminal coach. It watches commands typed in the shell and
occasionally suggests a cleaner way to write the same command, e.g.
`cat app.log | grep ERROR` -> `grep ERROR app.log`.

## Product principles

- **Silence over a questionable suggestion.** A missed tip is fine; a semantically wrong tip is not.
- No AI: no LLM and no AI APIs. Suggestions come only from hand-written, deterministic rules.
- A tip is shown every time a command matches a rule. State never stores command text.
- tipun updates itself: `analyze` starts a detached `tipun update --quiet` at most once a day.
  It only updates copies installed by `install.sh`, running the `install.sh` from the new release's
  tag, never from `main`. `analyze` never waits on the network.
- Fast: `tipun` runs before every prompt, so it must feel instant.
- tipun supports zsh and bash; the Rust core must stay shell-agnostic.

## Architecture

```
shell adapter (thin: zsh passes the raw and alias-expanded command, bash the raw command
  from history and its `alias` output; both pass the exit code)
  -> tipun analyze
  -> lexer -> parser -> Chain of pipelines (joined by &&)
  -> rules: fn(&Pipeline) -> Option<Rewrite>   (pure, no I/O), tried on each pipeline up to
     the first shell-state builtin; rules that merge stages only on the last one
  -> one short tip on stderr
```

- The core never depends on zsh or bash concepts. The shell adapter makes no rule decisions.
- **The parser is hand-written and fail-closed.** It accepts only a small whitelisted subset (plain
  words, single/double quotes without expansions, `|`, `&&`, `>`, `>>`, `<`, `n>`, `n>&m`).
  Anything else makes the whole command unsupported, and tipun stays silent. No regex-based
  matching. `;`, `||` and `&` stay unsupported: `a; b` runs `b` even when `a` fails.
  Control characters and bidi overrides are rejected, so a printed suggestion cannot drive the
  terminal.
- Words keep both `raw` (as typed, quotes included) and `value` (unquoted). Suggestions are
  rendered from `raw` so the user's quoting is preserved.
- Rules are plain functions in a static list. Command knowledge stays local to each rule; there is
  no global capability database, trait hierarchy or DSL.

## Rule safety

Equivalence contract: if the original exits 0 on regular, readable text files, the suggestion
produces byte-identical stdout, the same file side effects and exit 0. Only stderr wording and
error-path behavior may differ, plus exceptions explicitly documented on a rule. File side effects
are files created, written, truncated or deleted; access times are not part of the contract.

Every rule MUST have:
- preconditions checked in code: anything unknown (unexpected flag, long option, unusual
  redirect) returns `None`. No precondition comment; the code and the silent-test table are the
  checklist
- positive, negative, edge-case and intentionally-silent unit tests
- differential tests that run the original and the suggestion on fixtures and compare output

Known traps: `wc FILE` prints the filename (use `wc < FILE`); `sort -u` is not `sort | uniq` with
`-f`/`-n`/`-k`; `grep -c` is not `grep | wc -l` with `-o` or context flags; `grep -r` with no file
searches cwd; `tail -f`; `uniq A B` writes to B; `tr` takes no files; `cd foo` uses `CDPATH`;
aliases (`cat` -> `bat`) and prefixes (`sudo`, `time`); builtins earlier in a chain
(`exec > out`, `set`, `export`) that change how later commands run; merged stages in a chain
(`cat missing | sort && b` runs `b`, `sort missing && b` does not).

## CLI

- `tipun suggest <CMD>`: pure, prints the suggestion, exit 0 if found, 1 if silent
- `tipun analyze --command <CMD> [--expanded <CMD> | --aliases <TEXT>] [--exit-code <N>]
  [--columns <N>]`: hook entry point; draws a box when stderr is a terminal wide enough, plain
  lines otherwise; always exits 0 and never prints errors (only with `TIPUN_DEBUG=1`).
  `--aliases` takes the output of bash's `alias`; tipun expands command words itself and stays
  silent on aliases it cannot expand safely
- `tipun init zsh|bash`: prints the embedded shell integration
- `tipun update [--quiet]`: installs the latest release over a copy installed by `install.sh`

Env: `TIPUN_DISABLE`, `TIPUN_DEBUG`, `TIPUN_NO_UPDATE`, `TIPUN_STATE_DIR`. Silent and no update
check when `CI` is set; no tip when the exit code is non-zero. State lives in `$TIPUN_STATE_DIR`,
else `$XDG_STATE_HOME/tipun`, default `~/.local/state/tipun`: `last-update-check`, `install-path`
(written by `install.sh`) and `updated` (a one-time notice).

## Dependencies

Runtime: `clap`. Nothing else without a strong reason and the user's
agreement: no `regex`, `dirs`, `thiserror`, `anyhow`, `tracing`/`log`, shell-parser crates, async
runtimes or network crates. Dev dependencies: none unless clearly needed.

## Performance

- `analyze` reads only the update files in the state directory, spawns at most one detached
  updater a day and never scans history. Network access and other subprocesses belong in `update`.
- No regex, no large tables, no work at startup beyond argument parsing.
- Cap analyzed command length (1024 bytes).
- No threads or parallelism: each run handles one short command, and spawning would cost more
  than it saves.

## Working with the user

- The user is new to Rust. Explain Rust-specific concepts in the chat, not in code comments.
- Work in small steps. Add a file or module only when the current step uses it; never scaffold
  files ahead of time.
- No unused code: everything added must be reachable from `main` or a test in the same step, so
  the build has no `never used` warnings.
- Do not commit unless the user asks.
- If `cargo` is not on `PATH` in the agent shell, use `~/.cargo/bin/cargo`.
- This file is shared by all coding agents: `AGENTS.md` (Codex) is a symlink to it, and
  `.agents/skills` is a symlink to `.claude/skills`. Edit the originals and keep the wording
  tool-neutral.

## Code style

- Follow rustfmt defaults (4 spaces, 100-column lines) and idiomatic Rust naming.
- The best comment is no comment: write code that explains itself. Add a comment only where some
  unusual engineering was applied (a workaround, a non-obvious trap) that a reader could not work
  out from the code.
- No emoji or emoji-like symbols, except in tests of multibyte input.
- Prefer simple, explicit code over abstractions. No speculative generality.
- Plain data structs (such as the AST) may have public fields; no accessor boilerplate.
- Derive `Debug, Clone, PartialEq, Eq` where it makes sense.
- Prefer borrowing (`&str`, `&T`) over owned values; avoid needless allocations and clones.
- Match exhaustively; avoid catch-all `_` arms when the enum is ours.
- Imports ordered: std, external crates, local modules. No wildcard imports except `use super::*`
  in tests.

## Errors

- No `.unwrap()` outside tests. `.expect("reason")` only for true invariants.
- Fallible functions return `Result<T, E>` with small hand-written error types.
- `analyze` swallows all errors (bad arguments, parse failures) and exits 0 silently.

## Testing

- Tests that run `analyze` set `TIPUN_NO_UPDATE=1`, or `TIPUN_STATE_DIR` to a temp directory.
- Unit tests live in `#[cfg(test)] mod tests` in the same file; black-box tests of the binary go in
  `tests/`, using `env!("CARGO_BIN_EXE_tipun")`.
- Table-driven tests for the lexer, parser and rules, including inputs that must be rejected.

## Before committing

- [ ] `cargo test`
- [ ] `cargo build` with no warnings
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo fmt --check`
- [ ] No commented-out code, `dbg!` or debug prints
