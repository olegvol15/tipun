#!/bin/sh
# Install tipun from a GitHub release.
#
#   curl -fsSL https://raw.githubusercontent.com/olegvol15/tipun/main/install.sh | sh
#
# TIPUN_VERSION picks a release tag (default: the latest release).
# TIPUN_INSTALL_DIR picks the target directory (default: ~/.local/bin).
# TIPUN_STATE_DIR picks where the install location is recorded for updates.

set -eu

repo="olegvol15/tipun"

fail() {
    echo "tipun install: $*" >&2
    exit 1
}

main() {
    command -v curl > /dev/null 2>&1 || fail "curl is required"

    case "$(uname -s)" in
        Darwin) os="apple-darwin" ;;
        Linux) os="unknown-linux-musl" ;;
        *) fail "unsupported system: $(uname -s)" ;;
    esac
    case "$(uname -m)" in
        arm64 | aarch64) arch="aarch64" ;;
        x86_64 | amd64) arch="x86_64" ;;
        *) fail "unsupported architecture: $(uname -m)" ;;
    esac

    tag="${TIPUN_VERSION:-}"
    if [ -z "$tag" ]; then
        # The latest release page redirects to .../releases/tag/vX.Y.Z, which avoids the
        # rate-limited GitHub API.
        latest=$(curl -fsSLI -o /dev/null -w '%{url_effective}' \
            "https://github.com/$repo/releases/latest") || fail "could not find the latest release"
        tag="${latest##*/}"
    fi
    case "$tag" in
        v[0-9]*) ;;
        *) fail "unexpected release tag: $tag" ;;
    esac

    name="tipun-${tag#v}-$arch-$os"
    url="https://github.com/$repo/releases/download/$tag/$name.tar.gz"
    dir="${TIPUN_INSTALL_DIR:-$HOME/.local/bin}"

    tmp=$(mktemp -d)
    trap 'rm -rf "$tmp"' EXIT

    echo "Downloading tipun $tag for $arch-$os"
    curl -fsSL -o "$tmp/$name.tar.gz" "$url" || fail "could not download $url"
    curl -fsSL -o "$tmp/$name.tar.gz.sha256" "$url.sha256" || fail "could not download $url.sha256"

    expected=$(cut -d ' ' -f 1 < "$tmp/$name.tar.gz.sha256")
    actual=$(sha256 "$tmp/$name.tar.gz")
    if [ -z "$expected" ] || [ "$actual" != "$expected" ]; then
        fail "checksum mismatch for $name.tar.gz"
    fi

    tar -xzf "$tmp/$name.tar.gz" -C "$tmp"
    mkdir -p "$dir"
    cp "$tmp/$name/tipun" "$dir/.tipun.$$"
    chmod 755 "$dir/.tipun.$$"
    mv "$dir/.tipun.$$" "$dir/tipun"
    echo "Installed $("$dir/tipun" --version) to $dir/tipun"

    state="${TIPUN_STATE_DIR:-${XDG_STATE_HOME:-$HOME/.local/state}/tipun}"
    mkdir -p "$state"
    (cd "$dir" && pwd -P) > "$state/install-path.$$"
    mv "$state/install-path.$$" "$state/install-path"

    case ":$PATH:" in
        *":$dir:"*) ;;
        *) echo "Add $dir to your PATH to run tipun." ;;
    esac
    echo "To enable tips in zsh, add this line to ~/.zshrc:"
    echo "  eval \"\$(tipun init zsh)\""
    echo "tipun updates itself once a day. Set TIPUN_NO_UPDATE=1 to turn this off."
}

sha256() {
    if command -v sha256sum > /dev/null 2>&1; then
        sha256sum "$1" | cut -d ' ' -f 1
    elif command -v shasum > /dev/null 2>&1; then
        shasum -a 256 "$1" | cut -d ' ' -f 1
    else
        fail "sha256sum or shasum is required"
    fi
}

# Everything runs from main, so a download cut off halfway through runs nothing.
main "$@"
