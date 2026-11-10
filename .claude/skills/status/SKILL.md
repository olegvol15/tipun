---
name: status
description: Summarize where tipun stands - latest release, what landed on main since it, open PRs, local work in progress, and the next planned step. Use when the user asks "where are we", "what's shipped", "what's next", or wants changelog data.
---

# Status

Collect (run `git fetch --tags -q` first):
- latest release: `git describe --tags --abbrev=0 origin/main`
- unreleased commits: `git log --oneline <tag>..origin/main`, grouped by conventional type
  (feat, fix, others); compare with `CHANGELOG.md` to spot missing entries
- open PRs: `gh pr list --json number,title,headRefName,isDraft`, plus `gh pr checks <n> --required`
- local state: current branch, `git status --short`, commits not pushed
- next step: the current plan, if the agent has one, else the `next-step` skill's roadmap

Report as four short sections: Released, Unreleased on main, Open PRs, Next. One line per item,
no commit hashes unless asked. Do not change anything.
