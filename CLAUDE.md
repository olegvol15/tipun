# tipun

A local, deterministic, non-AI terminal coach. It watches commands typed in the shell and
occasionally suggests a cleaner way to write the same command, e.g.
`cat app.log | grep ERROR` -> `grep ERROR app.log`.

## Product principles

- **Silence over a questionable suggestion.** A missed tip is fine; a semantically wrong tip is not.
- Local only: no network, no telemetry, no LLM, no external APIs. State never stores command text.
- Fast: `tipun` runs before every prompt, so it must feel instant.
- v0.1 supports zsh only, but the Rust core must stay shell-agnostic.

## Architecture

```
zsh adapter (thin: captures raw command, alias-expanded command, exit code)
  -> tipun analyze
  -> lexer -> parser -> Pipeline AST
  -> rules: fn(&Pipeline) -> Option<Rewrite>   (pure, no I/O)
  -> state + throttle                           (disk is touched only if a rewrite exists)
  -> one short tip on stderr
```

- The core never depends on zsh concepts. The shell adapter makes no rule decisions.
- **The parser is hand-written and fail-closed.** It accepts only a small whitelisted subset (plain
  words, single/double quotes without expansions, `|`, `>`, `>>`, `<`, `n>`, `n>&m`). Anything else
  makes the whole command unsupported, and tipun stays silent. No regex-based matching.
- Words keep both `raw` (as typed, quotes included) and `value` (unquoted). Suggestions are
  rendered from `raw` so the user's quoting is preserved.
- Rules are plain functions in a static list. Command knowledge stays local to each rule; there is
  no global capability database, trait hierarchy or DSL.

## Rule safety

Equivalence contract: if the original exits 0 on regular, readable text files, the suggestion
produces byte-identical stdout, the same file side effects and exit 0. Only stderr wording and
error-path behavior may differ, plus exceptions explicitly documented on a rule.

Every rule MUST have:
- a precondition checklist in its doc comment; anything unknown (unexpected flag, long option,
  unusual redirect) returns `None`
- positive, negative, edge-case and intentionally-silent unit tests
- differential tests that run the original and the suggestion on fixtures and compare output

Known traps: `wc FILE` prints the filename (use `wc < FILE`); `sort -u` is not `sort | uniq` with
`-f`/`-n`/`-k`; `grep -c` is not `grep | wc -l` with `-o` or context flags; `grep -r` with no file
searches cwd; `tail -f`; `uniq A B` writes to B; `tr` takes no files; `cd foo` uses `CDPATH`;
aliases (`cat` -> `bat`) and prefixes (`sudo`, `time`).

## CLI

- `tipun suggest <CMD>`: pure, prints the suggestion, exit 0 if found, 1 if silent
- `tipun analyze --command <CMD> [--expanded <CMD>] [--exit-code <N>]`: hook entry point; always
  exits 0 and never prints errors (only with `TIPUN_DEBUG=1`)
- `tipun init zsh`: prints the embedded zsh integration

Env: `TIPUN_DISABLE`, `TIPUN_STATE_DIR`, `TIPUN_DEBUG`. Silent when `CI` is set or the exit code is
non-zero. State: `$XDG_STATE_HOME/tipun/state.json`, default `~/.local/state/tipun/state.json`.

## Dependencies

Runtime: `clap`, `serde`, `serde_json`. Nothing else without a strong reason and the user's
agreement: no `regex`, `dirs`, `thiserror`, `anyhow`, `tracing`/`log`, shell-parser crates, async
runtimes or network crates. Dev dependencies: none unless clearly needed.

## Performance

- No filesystem access unless a rewrite was found; no subprocesses; no history scanning.
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

## Code style

- Follow rustfmt defaults (4 spaces, 100-column lines) and idiomatic Rust naming.
- Keep comments minimal. No comments that restate the code. A short doc comment is welcome where
  intent is not obvious, and every rule documents its preconditions.
- No emoji or emoji-like symbols, except in tests of multibyte input.
- Prefer simple, explicit code over abstractions. No speculative generality.
- Plain data structs (AST, state) may have public fields; no accessor boilerplate.
- Derive `Debug, Clone, PartialEq, Eq` where it makes sense.
- Prefer borrowing (`&str`, `&T`) over owned values; avoid needless allocations and clones.
- Match exhaustively; avoid catch-all `_` arms when the enum is ours.
- Imports ordered: std, external crates, local modules. No wildcard imports except `use super::*`
  in tests.

## Errors

- No `.unwrap()` outside tests. `.expect("reason")` only for true invariants.
- Fallible functions return `Result<T, E>` with small hand-written error types.
- `analyze` swallows all errors (missing/corrupt state, parse failures) and exits 0 silently.

## Testing

- Unit tests live in `#[cfg(test)] mod tests` in the same file; black-box tests of the binary go in
  `tests/`, using `env!("CARGO_BIN_EXE_tipun")`.
- Table-driven tests for the lexer, parser and rules, including inputs that must be rejected.
- Use `TIPUN_STATE_DIR` with a temp directory for anything touching state.

## Before committing

- [ ] `cargo test`
- [ ] `cargo build` with no warnings
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo fmt --check`
- [ ] No commented-out code, `dbg!` or debug prints
