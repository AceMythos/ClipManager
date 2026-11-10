#!/usr/bin/env bash
set -euo pipefail

REPO="AceMythos/ClipManager"
BIN="clipManager"
ASSET="clipManager-x86_64-unknown-linux-gnu.tar.gz"
PREFIX="${PREFIX:-/usr}"

info() { printf '==> %s\n' "$*"; }
die() { printf 'error: %s\n' "$*" >&2; exit 1; }

# Runtime dependencies
if ! command -v wl-paste >/dev/null 2>&1 || ! command -v notify-send >/dev/null 2>&1; then
    if command -v apt-get >/dev/null 2>&1; then
        pkgs="wl-clipboard libnotify-bin"
        info "Installing $pkgs"
        sudo apt-get install -y $pkgs
    elif command -v dnf >/dev/null 2>&1; then
        pkgs="wl-clipboard libnotify"
        info "Installing $pkgs"
        sudo dnf install -y $pkgs
    elif command -v pacman >/dev/null 2>&1; then
        pkgs="wl-clipboard libnotify"
        info "Installing $pkgs"
        sudo pacman -S --needed --noconfirm $pkgs
    else
        die "missing dependencies: wl-clipboard and libnotify"
    fi
fi

command -v curl >/dev/null 2>&1 || die "curl is required to download the release."

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

download() {
    local url="$1" dest="$2"
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
printf 'If ClipManager does not show up there, log out and back in and try again.\n'
printf 'Optional Super+V: add a COSMIC custom shortcut running: pkill -USR1 -x clipManager\n'
