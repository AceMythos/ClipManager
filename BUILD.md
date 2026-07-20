# Build & Install

## Build

```bash
cargo build --release
```

## Install (add to panel)

```bash
pkexec install -m 755 target/release/clipManager /usr/bin/clipManager
pkexec cp "$PWD/clipManager.desktop" /usr/share/applications/
```

> `pkexec` resets the working directory, so always run these from the project root or use absolute paths.

Then add via **COSMIC Settings > Desktop > Panel**.

## Requirements

- Rust toolchain (install via [rustup](https://rustup.rs))
- `wl-clipboard` (`wl-paste` / `wl-copy`)
- `libnotify` (`notify-send`)
