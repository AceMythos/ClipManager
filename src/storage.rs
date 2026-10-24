use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::app::{HistoryEntry, PanelDisplay};

const STATE_VERSION: u32 = 1;

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

fn config_file() -> PathBuf {
    data_dir().join("config.json")
}

/// User preferences, persisted separately from the clipboard history.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub panel_display: PanelDisplay,
}

#[derive(Serialize, Deserialize)]
struct SavedState {
    #[serde(default)]
    version: u32,
    entries: Vec<HistoryEntry>,
}

pub async fn load_history() -> Vec<HistoryEntry> {
    match tokio::task::spawn_blocking(load_history_sync).await {
        Ok(entries) => entries,
        Err(e) => {
            tracing::error!("history load task panicked: {e}");
            Vec::new()
        }
    }
}

fn load_history_sync() -> Vec<HistoryEntry> {
    let path = data_file();

    match try_load_from(&path) {
        Ok(Some(entries)) => return entries,
        Ok(None) => return Vec::new(),
        Err(e) => tracing::warn!("failed to load history: {e}"),
    }

    let bak = data_file_bak();
    match try_load_from(&bak) {
        Ok(Some(entries)) => {
            tracing::warn!("restored {} entries from backup", entries.len());
            return entries;
        }
        Ok(None) => tracing::warn!("no backup file found"),
        Err(e) => tracing::error!("backup also failed: {e}"),
    }

    Vec::new()
}

fn try_load_from(path: &Path) -> Result<Option<Vec<HistoryEntry>>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let json = fs::read_to_string(path).map_err(|e| format!("cannot read: {e}"))?;
    Ok(Some(parse_state(&json)?))
}

/// Reject history written by a newer build instead of silently dropping fields
/// this build does not understand. The caller then falls back to the backup.
fn parse_state(json: &str) -> Result<Vec<HistoryEntry>, String> {
    let state = serde_json::from_str::<SavedState>(json).map_err(|e| format!("invalid JSON: {e}"))?;
    if state.version > STATE_VERSION {
        return Err(format!(
            "history version {} is newer than supported version {STATE_VERSION}",
            state.version
        ));
    }
    Ok(state.entries)
}

pub async fn save_history(entries: &[HistoryEntry]) {
    let entries = entries.to_vec();
    match tokio::task::spawn_blocking(move || {
        save_history_sync(&entries);
    })
    .await
    {
        Ok(()) => {}
        Err(e) => tracing::error!("history save task panicked: {e}"),
    }
}

fn save_history_sync(entries: &[HistoryEntry]) {
    let dir = data_dir();
    if let Err(e) = fs::create_dir_all(&dir) {
        tracing::error!("failed to create data dir {}: {e}", dir.display());
        return;
    }

    let path = data_file();
    let tmp_path = dir.join("history.json.tmp");

    let bak = data_file_bak();
    // `try_load_from` only returns `Ok(None)` when the file is absent, so there
    // is nothing to back up in that case.
    let backupable = match try_load_from(&path) {
        Ok(Some(_)) => true,
        Ok(None) => false,
        Err(e) => {
            tracing::warn!("cannot inspect {} for backup: {e}", path.display());
            false
        }
    };
    if backupable {
        if let Err(e) = fs::copy(&path, &bak) {
            tracing::warn!("failed to create backup {}: {e}", bak.display());
        }
    }

    let state = SavedState {
        version: STATE_VERSION,
        entries: entries.to_vec(),
    };

    match serde_json::to_string_pretty(&state) {
        Ok(json) => {
            if let Err(e) = fs::write(&tmp_path, &json) {
                tracing::error!("failed to write {}: {e}", tmp_path.display());
                return;
            }
            if let Err(e) = fs::rename(&tmp_path, &path) {
                tracing::error!("failed to rename {} -> {}: {e}", tmp_path.display(), path.display());
            }
        }
        Err(e) => tracing::error!("failed to serialize history: {e}"),
    }
}

pub async fn load_config() -> Config {
    match tokio::task::spawn_blocking(load_config_sync).await {
        Ok(config) => config,
        Err(e) => {
            tracing::error!("config load task panicked: {e}");
            Config::default()
        }
    }
}

fn load_config_sync() -> Config {
    let path = config_file();
    let Ok(json) = fs::read_to_string(&path) else {
        return Config::default();
    };
    match serde_json::from_str(&json) {
        Ok(config) => config,
        Err(e) => {
            tracing::warn!("invalid config {}: {e}", path.display());
            Config::default()
        }
    }
}

pub async fn save_config(config: Config) {
    match tokio::task::spawn_blocking(move || save_config_sync(config)).await {
        Ok(()) => {}
        Err(e) => tracing::error!("config save task panicked: {e}"),
    }
}

fn save_config_sync(config: Config) {
    let dir = data_dir();
    if let Err(e) = fs::create_dir_all(&dir) {
        tracing::error!("failed to create data dir {}: {e}", dir.display());
        return;
    }

    let path = config_file();
    let tmp_path = dir.join("config.json.tmp");
    match serde_json::to_string_pretty(&config) {
        Ok(json) => {
            if let Err(e) = fs::write(&tmp_path, &json) {
                tracing::error!("failed to write {}: {e}", tmp_path.display());
                return;
            }
            if let Err(e) = fs::rename(&tmp_path, &path) {
                tracing::error!(
                    "failed to rename {} -> {}: {e}",
                    tmp_path.display(),
                    path.display()
                );
            }
        }
        Err(e) => tracing::error!("failed to serialize config: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_state;

    #[test]
    fn legacy_history_without_version_loads() {
        assert!(parse_state(r#"{"entries":[]}"#).is_ok());
    }

    #[test]
    fn current_version_loads() {
        assert!(parse_state(r#"{"version":1,"entries":[]}"#).is_ok());
    }

    #[test]
    fn future_version_is_rejected() {
        let err = parse_state(r#"{"version":99,"entries":[]}"#).unwrap_err();
        assert!(err.contains("newer than supported"), "got: {err}");
    }

    #[test]
    fn corrupt_json_is_rejected() {
        assert!(parse_state("{not json").is_err());
    }
}
