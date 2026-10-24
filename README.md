# ClipManager

Clipboard history for the COSMIC panel. Copy something, click the icon, it's there.

## Features

- Panel icon shows a preview of your last copy — switch to icon-only in Settings
- Searchable history popup
- Click an entry to copy it back
- Pin entries to keep them forever
- Private mode — stops recording
- Unpinned entries vanish after 48 hours
- Keyboard navigation (see below)

**Pinned = permanent.** They survive the 48h cleanup, the 10,000-entry cap, and Clear All. To delete one, unpin it first.

Images and file copies are **not** recorded, but they're left untouched so pasting still works.

## Keyboard

| Key | Action |
| --- | --- |
| `↑` / `↓` | Move selection |
| `Ctrl+P` / `Ctrl+N` | Move selection |
| `Enter` | Copy the selected entry (or the first result) |
| `Ctrl+F` | Focus the search field |
| `Esc` | Clear search, then close the popup |

Click the gear in the popup footer to change the panel display mode.

## Install

```bash
cargo build --release
pkexec install -m 755 "$PWD/target/release/clipManager" /usr/bin/clipManager
pkexec install -m 644 "$PWD/clipManager.desktop" /usr/share/applications/clipManager.desktop
```

Then add it in **COSMIC Settings → Desktop → Panel**.

> Using `pkexec`? It resets the working directory — use absolute paths.

## Needs

- Rust ([rustup.rs](https://rustup.rs))
- `wl-clipboard` 2.2+ (`wl-paste`, `wl-copy`)
- `libnotify` (`notify-send`)

## Notes

History lives in `~/.local/share/com.github.igris.ClipManager/history.json`.
Settings live in `config.json` in the same directory. Delete them to reset.
Private mode resets on restart.

```bash
cargo test    # 19 tests
```
