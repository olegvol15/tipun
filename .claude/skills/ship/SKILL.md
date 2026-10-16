---
name: ship
description: Take local changes in tipun all the way to an open pull request - branch, checks, commit, push, PR, CI status - and clean up after the user merges. Use when the user says "ship it", "open a PR for this", or asks to get changes onto main. Also use for post-merge cleanup.
---

# Ship

`main` is protected: no direct pushes, every change lands through a squash-merged PR with green
CI and resolved review threads. The user merges; this skill never merges unless asked.

## Steps

1. **Look at the state:** `git status --short`, `git branch --show-current`, and whether the
   branch already has a PR (`gh pr view --json number,url,state`).
2. **Branch.** If on `main`, create a branch from an up-to-date `main`:
   `git switch main && git pull --ff-only && git switch -c <type>/<short-name>`, where `<type>` is
   the conventional commit type (`feat/useless-cat-rule`, `ci/release`, `docs/readme`). Uncommitted
   changes come along. If already on a feature branch, keep it.
3. **Check.** Run the `check` skill when Rust code changed, `actionlint` when workflows changed.
   Stop and report if anything fails.
4. **Commit** with the `commit` skill. Split unrelated changes into separate commits or PRs.
5. **Push:** `git push -u origin <branch>`.
6. **Open the PR** if none exists: write the title and body with the `pr-description` skill and
   run `gh pr create --base main --head <branch> --title ... --body-file -`. If a PR exists, the
   push updates it; say so.
7. **Status.** Report the PR URL and the current checks (`gh pr checks <n>`). Do not block on long
   waits; if checks are still pending, say so and offer to check again or run `address-review`
   once CodeRabbit has commented.

## After the user merges

When the user says the PR is merged:
1. Confirm: `gh pr view <n> --json state,mergeCommit`.
2. `git switch main && git pull --ff-only && git fetch --prune`
3. Delete the local branch (`git branch -D <branch>`; squash merges make `-d` refuse). The remote
   branch is deleted by GitHub automatically.
4. Show `git log --oneline -3` so the user sees the squashed commit.

## Never

- push to `main`, force-push, or rewrite published history
- merge, close or approve PRs without being asked
- skip failing checks or resolve review threads to make a PR look green
