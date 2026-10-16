---
name: pr-description
description: Write a pull request title and description for the current tipun branch, and open the PR only if asked. Use when the user asks for a PR description, PR text, or to open a pull request.
---

# PR description

## Steps

1. Find the base branch (usually `main`) and gather the changes on this branch:
   - `git log --oneline <base>..HEAD`
   - `git diff --stat <base>...HEAD`
   - `git diff <base>...HEAD`
   If the branch is `main` or has no commits beyond the base, say so and stop.
2. Read the diff, not just the commit messages. The description must match the code.
3. Write the title and body (format below).
4. Show the result to the user. Run `gh pr create` only if the user asked to open the PR, and
   only if a remote exists (`git remote -v`). Pass the body with a heredoc:
   ```
   gh pr create --title "<title>" --body-file - <<'EOF'
   <body>
   EOF
   ```
   Never push or open a PR without being asked.

## Title

Conventional Commits style, at most 72 characters, for example
`feat(rules): add useless-cat rule for grep, head and tail`. If the branch mixes types, use the
most significant one.

## Body

```markdown
## Summary

<2-4 sentences: what this PR does and why. Plain language, no file lists.>

## Changes

- <one bullet per meaningful change, grouped by area: cli, lexer, parser, rules, state, zsh>

## Safety

<Only when rules, the lexer or the parser change. Otherwise omit this section.>
- New or changed suggestions, each with one example: `cat f | grep x` -> `grep x f`
- Preconditions, and the cases that intentionally stay silent
- Documented exceptions to the equivalence contract, or "none"

## Testing

- <tests added or changed, with counts>
- <results of fmt, clippy, build, test>
- <manual checks, e.g. `tipun suggest '...'` output>

## Notes

<Optional: follow-ups, known gaps, anything reviewers should look at closely.>

<attribution line your agent tool asks for, if any>
```

## Rules

- Keep it short. Leave out sections that would be empty, except Summary and Testing.
- State only what the diff shows. If tests were not run, say so; do not claim they pass.
- Call out anything that affects users directly: CLI flags, env vars, the state file format, the
  zsh integration, new dependencies, startup time.
- No emoji, except in an attribution line your agent tool requires.
