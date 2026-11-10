#!/usr/bin/env bash
set -euo pipefail

REPO="AceMythos/ClipManager"
BIN="clipManager"
ASSET="clipManager-x86_64-unknown-linux-gnu.tar.gz"
PREFIX="${PREFIX:-/usr}"

info() { printf '==> %s\n' "$*"; }
die() { printf 'error: %s\n' "$*" >&2; exit 1; }

# Runtime dependencies
missing=()
command -v wl-paste >/dev/null 2>&1 || missing+=(wl-clipboard)
command -v notify-send >/dev/null 2>&1 || missing+=(libnotify-bin)
if [ "${#missing[@]}" -gt 0 ]; then
    if command -v apt-get >/dev/null 2>&1; then
        info "Installing ${missing[*]}"
        sudo apt-get install -y "${missing[@]}"
    else
        die "missing dependencies: ${missing[*]}"
    fi
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

download() {
    local url="$1" dest="$2"
    command -v curl >/dev/null 2>&1 || return 1
    curl -fsSL --retry 3 -o "$dest" "$url"
}

build_from_source() {
    command -v cargo >/dev/null 2>&1 || die "no prebuilt binary for $(uname -m) and Rust is not installed. Install it from https://rustup.rs and rerun."
    command -v git >/dev/null 2>&1 || die "git is required to build from source."
    info "Building from source (this takes a few minutes)"
    git clone --depth 1 "https://github.com/$REPO.git" "$tmp/src"
    (cd "$tmp/src" && cargo build --release --locked)
    cp "$tmp/src/target/release/$BIN" "$tmp/$BIN"
    cp "$tmp/src/clipManager.desktop" "$tmp/clipManager.desktop"
}

if [ "$(uname -m)" = "x86_64" ] && download "https://github.com/$REPO/releases/latest/download/$ASSET" "$tmp/$ASSET"; then
    download "https://github.com/$REPO/releases/latest/download/$ASSET.sha256" "$tmp/$ASSET.sha256" \
        || die "downloaded $ASSET but its checksum is missing"
    (cd "$tmp" && sha256sum -c "$ASSET.sha256")
    tar -xzf "$tmp/$ASSET" -C "$tmp"
    info "Using prebuilt release"
else
    build_from_source
fi

mkdir -p "$PREFIX/bin" "$PREFIX/share/applications" 2>/dev/null || true
sudo_cmd=""
if [ ! -w "$PREFIX/bin" ] || [ ! -w "$PREFIX/share/applications" ]; then
    sudo_cmd="sudo"
fi

$sudo_cmd install -Dm755 "$tmp/$BIN" "$PREFIX/bin/$BIN"
$sudo_cmd install -Dm644 "$tmp/clipManager.desktop" "$PREFIX/share/applications/clipManager.desktop"

info "Installed $PREFIX/bin/$BIN"
printf '\nNext: COSMIC Settings -> Desktop -> Panel -> Add applet -> ClipManager\n'
printf 'Restart the panel (or log out and back in) if ClipManager was already running.\n'
printf 'Optional Super+V: add a COSMIC custom shortcut running: pkill -USR1 -x clipManager\n'
