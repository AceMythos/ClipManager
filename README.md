# ClipManager

A clipboard history applet for the [COSMIC](https://system76.com/cosmic) desktop environment.

Built with [libcosmic](https://github.com/pop-os/libcosmic) (Rust).

## Features

- Clipboard history with up to 10,000 entries
- Search through clipboard history
- Pin important items to keep them at the top
- Private mode to pause history tracking
- Auto-detect content type (URL, code, color, email, text)
- Desktop notifications on copy
- Keyboard-friendly popup UI

## Dependencies

- `wl-paste` / `wl-copy` (from `wl-clipboard`)
- `notify-send` (for notifications)

## Build & Install

```bash
# Build release
cargo build --release

# Install binary
sudo install -m 755 target/release/clipManager /usr/bin/clipManager

# Install desktop file
sudo cp clipManager.desktop /usr/share/applications/
```

## Add to Panel

Open **COSMIC Settings > Desktop > Panel** and add the applet.

## License

MIT
