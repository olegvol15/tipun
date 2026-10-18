---
name: rust-explain
description: Explain Rust code or a Rust compiler error in tipun to a Rust beginner, and suggest the smallest fix for an error. Use when the user asks what some Rust code means, why the compiler complains, or pastes an error message.
---

# Rust explain

The user is new to Rust. Teach, don't lecture. Explain only; change code only if the user asks.

## Steps

1. Work out what to explain:
   - A pasted compiler or clippy error: if the user did not paste the full error, run
     `cargo build` (or `~/.cargo/bin/cargo build`) to get it.
   - A file, function or line: read it.
   - A concept ("what is `&str`?"): find a place in tipun's code that uses it, so the example is
     real.
2. Explain at the level of the code in front of the user:
   - What the code does, in one or two plain sentences.
   - The Rust concepts it relies on, each in a sentence or two, tied to the actual line. Typical
     ones: ownership and moves, borrowing (`&`, `&mut`), `String` vs `&str`, `Option` and
     `Result`, `?`, `match` and `if let`, `impl` and traits, `derive`, iterators and closures,
     lifetimes, modules and `pub`.
   - Why Rust works this way, briefly, when it helps the idea stick.
3. For an error:
   - Quote the key part of the message and translate it into plain language.
   - Point to the exact line and say what the compiler expected versus what it got.
   - Give the smallest fix as a short snippet, and say why it works. If there are two reasonable
     fixes, show the better one and mention the other in one line.

## Style

- Short. Use small code snippets taken from or modelled on the user's code.
- Introduce at most two or three new concepts at once; offer to go deeper instead of front-loading.
- No unrelated advice or refactors.
- Do not add explanatory comments to the code itself; `CLAUDE.md` allows comments only for
  unusual engineering.
