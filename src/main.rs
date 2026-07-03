//! kusage - a local, fast CLI that displays Kiro CLI usage metrics with a
//! compact, professional TUI-style dashboard.
//!
//! Reads Kiro CLI session sidecar files (`~/.kiro/sessions/cli/*.json`),
//! aggregates usage (credits, turns, requests, tool uses) by model, project,
//! and day, and renders a print-once dashboard. All processing is local: no
//! network calls, no telemetry.

mod aggregate;
mod loader;
mod model;
mod output;
mod render;
mod style;

use anyhow::Result;
use clap::Parser;
use time::{Duration, OffsetDateTime};

use crate::style::ColorMode;

/// Display Kiro CLI usage metrics from local session data.
#[derive(Parser, Debug)]
#[command(name = "kusage", version, about, long_about = None)]
struct Cli {
    /// Only include usage from the last N days.
    #[arg(long, value_name = "DAYS")]
    since: Option<i64>,

    /// Limit ranked breakdowns and the recent feed to the top N entries.
    #[arg(long, default_value_t = 10, value_name = "N")]
    top: usize,

    /// Emit machine-readable JSON instead of the dashboard.
    #[arg(long)]
    json: bool,

    /// Disable colors and styling (plain text).
    #[arg(long)]
    plain: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let dir = loader::sessions_dir()?;
    let (sessions, skipped) = loader::load_sessions(&dir)?;

    // Resolve the --since window to an absolute cutoff.
    let since = cli.since.map(|days| {
        let now = OffsetDateTime::now_utc();
        now - Duration::days(days)
    });

    let report = aggregate::aggregate(&sessions, since, cli.top.max(1));

    if cli.json {
        println!("{}", output::to_json(&report)?);
        return Ok(());
    }

    if report.summary.sessions == 0 {
        eprintln!(
            "No Kiro CLI usage found under {}.\n\
             Set KIRO_DIR to point at your Kiro home if it lives elsewhere.",
            dir.display()
        );
        return Ok(());
    }

    let mode = ColorMode::detect(cli.plain);
    print!("{}", render::render(&report, mode));

    if skipped > 0 {
        eprintln!("note: skipped {skipped} session file(s) that could not be parsed");
    }

    Ok(())
}
