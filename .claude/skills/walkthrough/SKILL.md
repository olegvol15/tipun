---
name: walkthrough
description: Walk through a tipun module or the whole pipeline step by step on a concrete input, showing the value at each stage. Use when the user wants to understand how some code works, especially code an AI agent wrote, or asks "how does the lexer/parser/rule handle X".
---

# Walkthrough

Help the user understand existing tipun code by tracing a real input through it. Read-only: do
not change code.

## Steps

1. Pick the scope and the input:
   - Scope: the module or function the user named, or the whole flow
     (lexer -> parser -> rules -> output) if they asked generally.
   - Input: the user's command if given; otherwise a small representative one, such as
     `cat "my file.log" | grep -i ERROR`. Choose one that exercises the interesting branches.
2. Read the code in scope. Only describe code that exists; if a stage is not written yet, say so
   and stop there.
3. Trace the input in order. For each step:
   - Name the function and point to it as `file:line`.
   - Show the relevant code (a few lines, not whole files).
   - Show the value going in and coming out, written as Rust values, e.g.
     `Word { raw: "\"my file.log\"", value: "my file.log" }`.
   - One sentence on what decision the code made and why.
4. If the trace hits a branch that rejects the input (returns `Unsupported` or `None`), stop and
   explain which check fired and what the user would see.
5. If it helps, run the binary (`cargo run -q -- suggest '<input>'`) or the module's tests to
   confirm the trace matches reality.

## Finish

- A three to five line recap of the whole path.
- Optionally one "try this next" input that takes a different branch, so the user can see the
  contrast.
- Point out any Rust idiom in the traced code the user may not know, in a sentence each. For
  deeper explanations, suggest the `rust-explain` skill.
