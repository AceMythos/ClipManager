# ClipManager

Clipboard history for the COSMIC panel. Copy something, click the icon, it's there.

## Features

- Panel icon shows a preview of your last copy
- Searchable history popup
- Click an entry to copy it back
- Pin entries to keep them forever
- Private mode — stops recording
- Unpinned entries vanish after 48 hours

**Pinned = permanent.** They survive the 48h cleanup, the 10,000-entry cap, and Clear All. To delete one, unpin it first.

Images and file copies are **not** recorded, but they're left untouched so pasting still works.

## Install

```bash
cargo build --release
sudo install -m 755 target/release/clipManager /usr/bin/clipManager
sudo cp "$PWD/clipManager.desktop" /usr/share/applications/
```

Then add it in **COSMIC Settings → Desktop → Panel**.

> Using `pkexec`? It resets the working directory — use absolute paths.

## Needs

- Rust ([rustup.rs](https://rustup.rs))
- `wl-clipboard` 2.2+ (`wl-paste`, `wl-copy`)
- `libnotify` (`notify-send`)

## Notes

History lives in `~/.local/share/com.github.igris.ClipManager/history.json`.
Delete it to reset. Private mode resets on restart.

```bash
cargo test    # 11 tests
```
