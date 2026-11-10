---
name: perf
description: Measure how fast tipun analyze runs and how big the binary is, and compare the current branch with main. Use after adding rules or touching startup code, before a release, or when the user asks if tipun is still fast.
---

# Perf

`analyze` runs before every prompt, so each run must stay a few milliseconds.

1. Build both versions in release mode: the working tree (`cargo build --release`) and main in a
   temporary worktree in the scratch directory (`git worktree add <dir> origin/main`).
2. Time each binary with `TIPUN_NO_UPDATE=1`, 500 runs per command, using a shell loop and `time`
   (no new tools or dependencies):
   - a matching command: `analyze --command='cat app.log | grep ERROR' --exit-code=0`
   - a silent command: `analyze --command='ls' --exit-code=0`
   - a 1024-byte command (the length cap)
   - an update check: a temp `TIPUN_STATE_DIR` with a fresh `last-update-check` stamp
3. Run each loop 3 times and keep the fastest, to reduce noise.
4. Report a table: case, main ms/run, branch ms/run, change; plus binary size for both.
   Flag any case more than 20% slower or above 10 ms/run, and point to the likely cause in the diff.
5. Remove the worktree (`git worktree remove <dir>`). Do not change code unless the user asks.
