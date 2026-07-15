# ClipManager

Clipboard history applet for COSMIC desktop panel.

## What It Does

When you copy something, the panel icon shows a preview. Click it to open a popup with your full clipboard history.

**Popup features:**
- Search bar to filter entries
- Click any entry to copy it back
- Pin items (star icon) to keep them at the top
- Delete individual entries or clear all
- Private mode toggle — stops recording new copies
- Auto-clear: entries older than 48 hours are automatically removed (pinned entries are kept)

**Panel icon** shows a truncated preview of your last copy.

## Install

```bash
cargo build --release
sudo install -m 755 target/release/clipManager /usr/bin/clipManager
sudo cp clipManager.desktop /usr/share/applications/
```

Then add via **COSMIC Settings > Desktop > Panel**.

## Requirements

- `wl-clipboard` (`wl-paste` / `wl-copy`)
- `libnotify` (`notify-send`)
