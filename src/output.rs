//! JSON output for scripting (`--json`).
//!
//! Serializes the aggregated report into a stable, machine-friendly shape that
//! is independent of the terminal renderer. Timestamps are emitted as RFC 3339
//! strings; dates as `YYYY-MM-DD`.

use serde::Serialize;

use crate::aggregate::{Report, Status};

#[derive(Serialize)]
struct JsonReport {
    summary: JsonSummary,
    by_model: Vec<JsonGroup>,
    by_project: Vec<JsonGroup>,
    by_day: Vec<JsonDay>,
    recent: Vec<JsonRecent>,
}

#[derive(Serialize)]
struct JsonSummary {
    sessions: usize,
    turns: usize,
    requests: u64,
    tool_uses: u64,
    credits: f64,
    input_tokens: u64,
    output_tokens: u64,
    duration_secs: f64,
    first_day: Option<String>,
    last_day: Option<String>,
}

#[derive(Serialize)]
struct JsonGroup {
    label: String,
    sessions: usize,
    turns: usize,
    credits: f64,
    pct: f64,
}

#[derive(Serialize)]
struct JsonDay {
    date: String,
    credits: f64,
    sessions: usize,
}

#[derive(Serialize)]
struct JsonRecent {
    title: String,
    credits: f64,
    status: &'static str,
    delta_pct: Option<f64>,
    when: Option<String>,
}

fn status_str(s: Status) -> &'static str {
    match s {
        Status::Ok => "ok",
        Status::Cancelled => "cancelled",
        Status::Error => "error",
    }
}

/// Serialize the report as pretty-printed JSON.
pub fn to_json(report: &Report) -> anyhow::Result<String> {
    let rfc3339 = &time::format_description::well_known::Rfc3339;
    let jr = JsonReport {
        summary: JsonSummary {
            sessions: report.summary.sessions,
            turns: report.summary.turns,
            requests: report.summary.requests,
            tool_uses: report.summary.tool_uses,
            credits: report.summary.credits,
            input_tokens: report.summary.input_tokens,
            output_tokens: report.summary.output_tokens,
            duration_secs: report.summary.duration_secs,
            first_day: report.summary.first_day.map(|d| d.to_string()),
            last_day: report.summary.last_day.map(|d| d.to_string()),
        },
        by_model: report.by_model.iter().map(group).collect(),
        by_project: report.by_project.iter().map(group).collect(),
        by_day: report
            .by_day
            .iter()
            .map(|d| JsonDay {
                date: d.date.to_string(),
                credits: d.credits,
                sessions: d.sessions,
            })
            .collect(),
        recent: report
            .recent
            .iter()
            .map(|r| JsonRecent {
                title: r.title.clone(),
                credits: r.credits,
                status: status_str(r.status),
                delta_pct: r.delta_pct,
                when: r.when.and_then(|t| t.format(rfc3339).ok()),
            })
            .collect(),
    };
    Ok(serde_json::to_string_pretty(&jr)?)
}

fn group(g: &crate::aggregate::GroupRow) -> JsonGroup {
    JsonGroup {
        label: g.label.clone(),
        sessions: g.sessions,
        turns: g.turns,
        credits: g.credits,
        pct: g.pct,
    }
}
