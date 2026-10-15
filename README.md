# tipun

A small terminal coach. tipun watches the commands you run in your shell and occasionally
suggests a cleaner way to write the same command.

```
$ cat app.log | grep ERROR
...
tipun tip: grep ERROR app.log
  grep can read the file directly, no need for cat.
```

## Principles

- **Local and private.** No AI, no network, no telemetry. Nothing leaves your machine.
- **Deterministic.** Suggestions come from parsing the command, not from guessing.
- **Silent when unsure.** tipun only suggests a rewrite when it is confident the behavior is the
  same. A missed tip is fine; a wrong one is not.
- **Not annoying.** Tips are rare, throttled, and the same tip is not repeated forever.

## Status

Early development. Nothing is usable yet. The first version will support zsh and a small set of
rewrites, such as:

| You type | tipun suggests |
|---|---|
| `cat file \| grep x` | `grep x file` |
| `cat file \| wc -l` | `wc -l < file` |
| `sort file \| uniq` | `sort -u file` |
| `grep x file \| wc -l` | `grep -c x file` |

## Building

Requires a Rust toolchain ([rustup](https://rustup.rs)).

```
cargo build
cargo test
```
