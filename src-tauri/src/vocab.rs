//! Personal canonical word list for code-switch correction (translit).
//!
//! Stored as `~/.nbp/vocab.txt` (one term per line) — same plain-file pattern as
//! pipelines/templates. The fluidaudio sidecar reads this path via `--vocab`;
//! the translit pass phonetically matches Cyrillic-mangled speech against these
//! canonical English terms (no per-mangling aliases needed).

use crate::config::get_config_dir;
use std::fs;
use std::path::PathBuf;

pub fn vocab_path() -> PathBuf {
    get_config_dir().join("vocab.txt")
}

/// Read the canonical term list (skips blanks and `#` comments).
#[tauri::command]
pub fn get_vocab() -> Vec<String> {
    let Ok(content) = fs::read_to_string(vocab_path()) else {
        return Vec::new();
    };
    content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|s| s.to_string())
        .collect()
}

/// Replace the canonical term list. Trims, drops blanks, one term per line.
#[tauri::command]
pub fn save_vocab(terms: Vec<String>) -> Result<(), String> {
    let dir = get_config_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| format!("Failed to create config dir: {}", e))?;
    }
    let cleaned: Vec<String> = terms
        .iter()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect();
    let body = if cleaned.is_empty() {
        String::new()
    } else {
        format!("{}\n", cleaned.join("\n"))
    };
    fs::write(vocab_path(), body).map_err(|e| format!("Failed to write vocab.txt: {}", e))?;
    Ok(())
}
