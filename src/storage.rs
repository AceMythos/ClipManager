use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::app::HistoryEntry;

fn data_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".local/share/com.github.igris.ClipManager")
}

fn data_file() -> PathBuf {
    data_dir().join("history.json")
}

fn data_file_bak() -> PathBuf {
    data_dir().join("history.json.bak")
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
struct SavedState {
    #[serde(default)]
    version: u32,
    entries: Vec<HistoryEntry>,
}

pub async fn load_history() -> Vec<HistoryEntry> {
    tokio::task::spawn_blocking(load_history_sync)
        .await
        .unwrap_or_default()
}

fn load_history_sync() -> Vec<HistoryEntry> {
    let path = data_file();

    match try_load_from(&path) {
        Ok(Some(entries)) => return entries,
        Ok(None) => return Vec::new(),
        Err(e) => eprintln!("clipboard-applet: failed to load history: {e}"),
    }

    let bak = data_file_bak();
    match try_load_from(&bak) {
        Ok(Some(entries)) => {
            eprintln!("clipboard-applet: restored {} entries from backup", entries.len());
            return entries;
        }
        Ok(None) => eprintln!("clipboard-applet: no backup file found"),
        Err(e) => eprintln!("clipboard-applet: backup also failed: {e}"),
    }

    Vec::new()
}

fn try_load_from(path: &Path) -> Result<Option<Vec<HistoryEntry>>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let json = fs::read_to_string(path).map_err(|e| format!("cannot read: {e}"))?;
    let state = serde_json::from_str::<SavedState>(&json).map_err(|e| format!("invalid JSON: {e}"))?;
    Ok(Some(state.entries))
}

pub async fn save_history(entries: &[HistoryEntry]) {
    let entries = entries.to_vec();
    let _ = tokio::task::spawn_blocking(move || {
        save_history_sync(&entries);
    })
    .await;
}

fn save_history_sync(entries: &[HistoryEntry]) {
    let dir = data_dir();
    if fs::create_dir_all(&dir).is_err() {
        return;
    }

    let path = data_file();
    let tmp_path = dir.join("history.json.tmp");

    let bak = data_file_bak();
    if let Ok(Some(_)) = try_load_from(&path) {
        let _ = fs::copy(&path, &bak);
    } else if path.exists() && !bak.exists() {
        let _ = fs::copy(&path, &bak);
    }

    let state = SavedState {
        version: 1,
        entries: entries.to_vec(),
    };

    if let Ok(json) = serde_json::to_string_pretty(&state) {
        if fs::write(&tmp_path, &json).is_ok() {
            let _ = fs::rename(&tmp_path, &path);
        }
    }
}
