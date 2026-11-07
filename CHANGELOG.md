# Changelog

All notable changes to tipun are documented in this file. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and tipun follows
[Semantic Versioning](https://semver.org/).

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

[0.1.0]: https://github.com/olegvol15/tipun/releases/tag/v0.1.0
