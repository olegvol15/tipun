# tipun

A small terminal coach. tipun watches the commands you run in your shell and occasionally
suggests a cleaner way to write the same command.

```
$ cat app.log | grep ERROR
...
tipun tip: grep ERROR app.log
  grep can read the file directly, no need for cat.
```

## Principles

- **No AI.** Suggestions come from hand-written rules, not from a model, and your commands never
  leave your machine.
- **Deterministic.** Suggestions come from parsing the command, not from guessing.
- **Silent when unsure.** tipun only suggests a rewrite when it is confident the behavior is the
  same. A missed tip is fine; a wrong one is not.
- **Short.** One tip on stderr after a command that could be written more simply, and nothing
  otherwise.

## Status

Early development, zsh only. tipun knows a small set of rewrites:

| You type | tipun suggests |
|---|---|
| `cat file \| grep x` | `grep x file` |
| `cat file \| wc -l` | `wc -l < file` |
| `sort file \| uniq` | `sort -u file` |
| `grep x file \| wc -l` | `grep -c x file` |
| `ls -l -a`, `rm -r -f dir` | `ls -la`, `rm -rf dir` |

## Install

On macOS or Linux:

```
curl -fsSL https://raw.githubusercontent.com/olegvol15/tipun/main/install.sh | sh
```

The script downloads the binary for your platform from the latest release, checks its SHA-256
checksum and installs it to `~/.local/bin`. Set `TIPUN_INSTALL_DIR` to install elsewhere, or
`TIPUN_VERSION` (for example `v0.1.0`) to pick a release.

To install by hand instead, download the archive for your platform from the
[latest release](https://github.com/olegvol15/tipun/releases/latest): macOS (`aarch64-apple-darwin`,
`x86_64-apple-darwin`) or Linux (`x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`).
For example, on an Apple Silicon Mac:

```
curl -LO https://github.com/olegvol15/tipun/releases/download/v0.1.0/tipun-0.1.0-aarch64-apple-darwin.tar.gz
shasum -a 256 tipun-0.1.0-aarch64-apple-darwin.tar.gz
tar -xzf tipun-0.1.0-aarch64-apple-darwin.tar.gz
mkdir -p ~/.local/bin
mv tipun-0.1.0-aarch64-apple-darwin/tipun ~/.local/bin/
```

Compare the checksum with the `.sha256` file next to the archive, and make sure `~/.local/bin` is
on your `PATH`. With a Rust toolchain you can instead build from a checkout:
`cargo install --path .`.

Then add the hook to `~/.zshrc`:

```
echo 'eval "$(tipun init zsh)"' >> ~/.zshrc
```

## Usage

Open a new shell and type, for example, `sort file | uniq`. tipun stays silent after failed
commands, when `CI` is set, and when aliases change the command (for example `alias cat=bat`).
Aliases that only add `--color=auto` or `--exclude-dir=...` to `grep`, like the one oh-my-zsh
defines, are fine.
Set `TIPUN_DISABLE=1` to turn it off, or `TIPUN_DEBUG=1` to see why it stayed silent.

## Updates

A copy installed with the install script updates itself. Once a day tipun checks GitHub for a
newer release in the background and installs it; the next prompt shows `tipun updated to vX.Y.Z`.
Run `tipun update` to update right away, or set `TIPUN_NO_UPDATE=1` to turn this off. Copies
installed with `cargo install` are not touched; update those with cargo.

To try a command without the hook:

```
tipun suggest 'cat app.log | grep ERROR'
```

## Building

Requires a Rust toolchain ([rustup](https://rustup.rs)).

```
cargo build
cargo test
```

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
tipun by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
