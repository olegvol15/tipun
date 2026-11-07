# Changelog

All notable changes to tipun are documented in this file. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and tipun follows
[Semantic Versioning](https://semver.org/).

## [0.1.1] - 2026-11-07

### Added

- One-line install: `curl -fsSL https://raw.githubusercontent.com/olegvol15/tipun/main/install.sh | sh`.
  It downloads the binary for your platform, verifies its SHA-256 checksum and installs it to
  `~/.local/bin`. `TIPUN_VERSION` and `TIPUN_INSTALL_DIR` choose the release and the directory.
- Automatic updates for copies installed with the install script: once a day tipun checks GitHub
  for a newer release in the background, installs it, and shows `tipun updated to vX.Y.Z` on the
  next prompt. `tipun update` updates right away; `TIPUN_NO_UPDATE=1` turns it off. Copies
  installed with v0.1.0 or `cargo install` do not update themselves; rerun the install line once.
- Bundle separate short flags: `ls -l -a` -> `ls -la`, `rm -r -f dir` -> `rm -rf dir`,
  `git commit -a -m msg` -> `git commit -am msg`, also for cp, mv, mkdir, touch and grep.

### Changed

- tipun now contacts GitHub once a day to check for updates. It still sends no command text and
  uses no AI.

### Fixed

- Grep tips now appear when `grep` is aliased to add only `--color=auto` or `--exclude-dir=...`,
  as oh-my-zsh does. Other aliases still keep tipun silent.

## [0.1.0] - 2026-11-07

### Added

- zsh integration: `eval "$(tipun init zsh)"` in `~/.zshrc` shows a tip on stderr after a command
  that could be written more simply. tipun stays silent after failed commands, when `CI` or
  `TIPUN_DISABLE` is set, and when aliases change the command (for example `alias cat=bat`).
- `tipun suggest <CMD>` prints the suggested rewrite of a command, or exits 1 when there is none.
- `TIPUN_DEBUG=1` explains why tipun stayed silent.
- Rewrite `cat FILE | grep/head/tail/sort/uniq ...` to pass the file directly, and
  `cat FILE | wc -l` or `cat FILE | tr ...` to read it with `<`.
- Rewrite `sort ... | uniq` to `sort -u ...`.
- Rewrite `grep ... | wc -l` at the end of a pipeline to `grep -c ...`. Unlike the original, the
  suggestion exits 1 when nothing matches, and on macOS the count is printed without padding.
- Output redirects such as `> out.txt` and `2>/dev/null` are kept in the suggestion. tipun stays
  silent when a redirect could change the result, such as writing to the file being read.
- Prebuilt binaries for macOS (arm64, x86_64) and Linux (musl, x86_64 and arm64).

[0.1.1]: https://github.com/olegvol15/tipun/releases/tag/v0.1.1
[0.1.0]: https://github.com/olegvol15/tipun/releases/tag/v0.1.0
