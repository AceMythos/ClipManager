# Build & Install

## Build

```bash
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
```

## Clean rebuild

Remove the installed binary and desktop entry first, then rebuild from scratch:

```bash
pkexec rm -f /usr/bin/clipManager
pkexec rm -f /usr/share/applications/clipManager.desktop
cargo clean
cargo build --release
```

> `pkexec` resets the working directory. Pass absolute paths to it, and run
> `cargo` separately from the project root as shown above.

## Install

```bash
pkexec install -m 755 "$PWD/target/release/clipManager" /usr/bin/clipManager
pkexec install -m 644 "$PWD/clipManager.desktop" /usr/share/applications/clipManager.desktop
```

Verify the installed files:

```bash
ls -l /usr/bin/clipManager /usr/share/applications/clipManager.desktop
```

Then add it via **COSMIC Settings > Desktop > Panel**, or restart the panel so it
picks up the new binary.

## Requirements

- Rust toolchain (install via [rustup](https://rustup.rs))
- `wl-clipboard` (`wl-paste` / `wl-copy`)
- `libnotify` (`notify-send`)
