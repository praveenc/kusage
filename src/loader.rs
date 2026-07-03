//! Discovery and loading of Kiro CLI session sidecar files.
//!
//! Kiro stores CLI sessions at `~/.kiro/sessions/cli/<uuid>.json`. This module
//! locates that directory (respecting a `KIRO_HOME`/`KIRO_DIR` override), reads
//! every `*.json` sidecar, and parses each into a [`Session`]. Malformed or
//! partial files are skipped rather than aborting the whole load, so one bad
//! file never blocks the report.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::model::Session;

/// Resolve the directory that holds Kiro CLI session sidecar files.
///
/// Resolution order:
/// 1. `KIRO_DIR` / `KIRO_HOME` environment variable, if set, joined with
///    `sessions/cli`.
/// 2. The default `~/.kiro/sessions/cli`.
pub fn sessions_dir() -> Result<PathBuf> {
    if let Some(root) = std::env::var_os("KIRO_DIR").or_else(|| std::env::var_os("KIRO_HOME")) {
        return Ok(Path::new(&root).join("sessions").join("cli"));
    }
    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(home.join(".kiro").join("sessions").join("cli"))
}

/// Load and parse every session sidecar under `dir`.
///
/// Returns the successfully parsed sessions along with a count of files that
/// were skipped due to read or parse errors. An empty or missing directory
/// yields an empty vector (not an error), so callers can render a friendly
/// "no data" state.
pub fn load_sessions(dir: &Path) -> Result<(Vec<Session>, usize)> {
    if !dir.exists() {
        return Ok((Vec::new(), 0));
    }

    let mut sessions = Vec::new();
    let mut skipped = 0usize;

    let entries = fs::read_dir(dir)
        .with_context(|| format!("failed to read sessions directory: {}", dir.display()))?;

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        match parse_session_file(&path) {
            Ok(session) => sessions.push(session),
            Err(_) => skipped += 1,
        }
    }

    Ok((sessions, skipped))
}

/// Read and parse a single session sidecar file.
fn parse_session_file(path: &Path) -> Result<Session> {
    let bytes = fs::read(path)
        .with_context(|| format!("failed to read session file: {}", path.display()))?;
    let session: Session = serde_json::from_slice(&bytes)
        .with_context(|| format!("failed to parse session file: {}", path.display()))?;
    Ok(session)
}
