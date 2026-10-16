---
name: release
description: Prepare and publish a tipun release - pick the version, bump Cargo.toml, update CHANGELOG.md from conventional commits, open a release PR, and after it is merged tag main so the release workflow publishes the binaries. Use when the user asks to cut, prepare or publish a release or version.
---

# Release

Publishing happens in `.github/workflows/release.yml`: pushing a tag `vX.Y.Z` that matches the
`Cargo.toml` version builds four binaries (macOS arm64/x86_64, Linux musl x86_64/arm64) and
creates a GitHub Release. `main` is protected, so the version bump goes through a PR first.

## Part 1: release PR

1. Start from an up-to-date `main` with a clean working tree. Find the last tag
   (`git describe --tags --abbrev=0`, none means first release) and list the commits since then
   (`git log <last-tag>..HEAD --oneline`, or all commits for the first release).
2. **Pick the version** from the commits, following semver. While tipun is `0.x`: any breaking
   change (`!` or `BREAKING CHANGE:`) bumps the minor version, anything else bumps the patch.
   From `1.0.0` on: breaking -> major, `feat` -> minor, otherwise patch. Show the reasoning and let
   the user override. Pre-releases use a suffix such as `0.2.0-rc.1` (published as a pre-release).
3. **Branch:** `git switch -c release/vX.Y.Z`.
4. **Bump** `version` in `Cargo.toml`, then run `cargo check` so `Cargo.lock` picks up the new
   version.
5. **Changelog:** add a section at the top of `CHANGELOG.md` (create the file if missing, in the
   Keep a Changelog style):
   ```
   ## [X.Y.Z] - YYYY-MM-DD
   ### Added      <- feat
   ### Changed    <- perf, refactor with user-visible effect, breaking changes (marked BREAKING)
   ### Fixed      <- fix
   ```
   One line per user-visible change, written for users, not copied commit subjects. Leave out
   `chore`, `ci`, `test`, `docs` and `build` unless users would care (for example a new install
   method).
6. Run the `check` skill, commit as `chore(release): vX.Y.Z` with the `commit` skill, and open
   the PR with the `ship` skill. The user reviews and merges.

## Part 2: tag and publish (after the PR is merged)

1. `git switch main && git pull --ff-only`, then confirm `Cargo.toml` on `main` has the new
   version and the release PR is merged.
2. Create an annotated tag on that commit: `git tag -a vX.Y.Z -m "tipun vX.Y.Z"`.
3. **Ask the user before pushing the tag.** Pushing it publishes a public release that cannot be
   cleanly undone. Then `git push origin vX.Y.Z`.
4. Watch the workflow (`gh run list --workflow release.yml --limit 1`, `gh run watch <id>`). If a
   job fails, report the error; do not delete or move the tag without the user's agreement.
5. Verify: `gh release view vX.Y.Z` lists four `.tar.gz` files and four `.sha256` files.
   Optionally download the macOS arm64 archive, check its checksum (`shasum -a 256 -c`) and run
   `tipun --version`.

## Report

The version and why, the changelog section, the PR link, then (after Part 2) the release URL and
the list of assets.
