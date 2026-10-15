---
name: commit
description: Create a git commit for tipun using the Conventional Commits format. Use when the user asks to commit, make a commit, or write a commit message.
---

# Commit

Commit only when the user asked for it. Never push.

## Steps

1. Run `git status --short`, `git diff` and `git diff --staged` to see what changed.
2. Decide what belongs in this commit:
   - If the user named files or a scope, commit only that.
   - If the changes cover unrelated things (for example a skill and a parser change), propose
     splitting them into separate commits and ask before committing.
   - Never stage secrets, `.env`, `target/` or editor files.
3. If Rust code is part of the commit, run the `check` skill first. If it fails, report the
   failures and ask whether to commit anyway.
4. Stage the files by name (`git add <paths>`). Avoid `git add -A` unless every change belongs.
5. Write the message (format below) and commit with a heredoc so line breaks survive:
   ```
   git commit -F - <<'EOF'
   <message>
   EOF
   ```
6. Show `git log --oneline -1` and `git status --short`, and mention anything left uncommitted.

## Message format

```
<type>(<scope>): <summary>

<body>

<footer>
```

**Type**, one of:
- `feat`: a new user-visible capability (a new rule, subcommand or option)
- `fix`: a bug fix, including a rule that suggested something wrong
- `refactor`: a code change with no behavior change
- `test`: tests only
- `docs`: README, CLAUDE.md, doc comments
- `perf`: faster or lighter with no behavior change
- `build`: Cargo.toml, dependencies, release profile
- `ci`: GitHub Actions
- `chore`: everything else (skills, tooling, cleanup)

**Scope** (optional, one word): `cli`, `lexer`, `parser`, `rules`, `state`, `zsh`, `skills`,
`deps`. Leave it out if the change spans several areas.

**Summary:**
- imperative mood ("add", not "added" or "adds"), lowercase first word, no trailing period
- at most 72 characters for the whole first line
- says what changes for tipun, not which files were edited

**Body** (skip it for trivial changes): wrap at 72 columns. Explain why the change is needed and
anything a reviewer would not see in the diff. For a new or changed rule, list its preconditions
and any documented exception to the equivalence contract.

**Footer:**
- `BREAKING CHANGE: <what breaks>` when CLI flags, env vars, the state file format or the zsh
  integration change incompatibly. Also add `!` after the type: `feat(cli)!: ...`.
- End every message with:
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`

## Examples

```
feat(lexer): tokenize words, quotes and pipes

Words keep both the raw text and the unquoted value so suggestions
can reuse the user's quoting. Anything outside the supported subset
makes the whole command unsupported and tipun stays silent.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```
fix(rules): keep grep-count silent when grep has -o

`grep -o` prints one line per match, so `wc -l` counts matches while
`grep -c` counts lines.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

```
chore(skills): add commit and pr-description skills

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```
