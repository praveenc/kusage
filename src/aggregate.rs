//! Aggregation of parsed sessions into the metrics the report renders.
//!
//! Everything here is pure: it takes `&[Session]` and produces owned summary
//! structs. Time handling uses the `end_timestamp` of each turn (falling back
//! to the session `updated_at`) to bucket usage by day and to apply the
//! `--since` time window.

use std::collections::HashMap;

use time::{Date, OffsetDateTime};

use crate::model::Session;

/// Status of a session or turn, derived from Kiro's `end_reason`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Turn completed normally (`UserTurnEnd`).
    Ok,
    /// Turn was cancelled or interrupted (`Cancelled`).
    Cancelled,
    /// Turn errored or a tool use was rejected (`Error`, `ToolUseRejected`).
    Error,
}

impl Status {
    fn from_end_reason(reason: Option<&str>) -> Status {
        match reason {
            Some("UserTurnEnd") => Status::Ok,
            Some("Cancelled") => Status::Cancelled,
            Some("Error") | Some("ToolUseRejected") => Status::Error,
            _ => Status::Ok,
        }
    }

    /// A compact glyph for the recent feed (✓ / ~ / ✗).
    pub fn glyph(&self) -> char {
        match self {
            Status::Ok => '✓',
            Status::Cancelled => '~',
            Status::Error => '✗',
        }
    }
}

/// Top-level totals across all included sessions.
#[derive(Debug, Default, Clone)]
pub struct Summary {
    pub sessions: usize,
    pub turns: usize,
    pub requests: u64,
    pub tool_uses: u64,
    pub credits: f64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    /// Total active duration across all turns, in seconds.
    pub duration_secs: f64,
    /// Earliest and latest day observed (for the header date range).
    pub first_day: Option<Date>,
    pub last_day: Option<Date>,
}

/// A ranked breakdown row (used for By Model, By Project).
#[derive(Debug, Clone)]
pub struct GroupRow {
    pub label: String,
    pub sessions: usize,
    pub turns: usize,
    pub credits: f64,
    /// Share of total credits, 0.0..=100.0.
    pub pct: f64,
}

/// Per-day usage, ordered oldest to newest.
#[derive(Debug, Clone)]
pub struct DayRow {
    pub date: Date,
    pub credits: f64,
    pub sessions: usize,
}

/// A recent session entry for the reverse-chronological feed.
#[derive(Debug, Clone)]
pub struct RecentRow {
    pub title: String,
    pub credits: f64,
    pub status: Status,
    /// Signed percent change in credits versus the previous session in the feed.
    pub delta_pct: Option<f64>,
    pub when: Option<OffsetDateTime>,
}

/// The full aggregated report handed to the renderer.
#[derive(Debug, Clone)]
pub struct Report {
    pub summary: Summary,
    pub by_model: Vec<GroupRow>,
    pub by_project: Vec<GroupRow>,
    pub by_day: Vec<DayRow>,
    pub recent: Vec<RecentRow>,
}

/// Parse an ISO-8601 timestamp Kiro emits (RFC 3339, e.g. `2026-07-02T18:38:27.469426Z`).
fn parse_ts(s: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339).ok()
}

/// Best available timestamp for a session: latest turn end, else `updated_at`,
/// else `created_at`.
fn session_time(session: &Session) -> Option<OffsetDateTime> {
    session
        .turns()
        .iter()
        .filter_map(|t| t.end_timestamp.as_deref())
        .filter_map(parse_ts)
        .max()
        .or_else(|| session.updated_at.as_deref().and_then(parse_ts))
        .or_else(|| session.created_at.as_deref().and_then(parse_ts))
}

/// Shorten a project path to a readable label (last two path components).
fn project_label(cwd: Option<&str>) -> String {
    let Some(cwd) = cwd else {
        return "(unknown)".to_string();
    };
    let parts: Vec<&str> = cwd.trim_end_matches('/').split('/').collect();
    match parts.as_slice() {
        [.., a, b] => format!("{a}/{b}"),
        [a] => a.to_string(),
        _ => cwd.to_string(),
    }
}

/// Aggregate sessions into a full report.
///
/// `since` optionally drops sessions whose activity is older than the cutoff.
/// `top_n` limits the By-X breakdowns and recent feed length.
pub fn aggregate(sessions: &[Session], since: Option<OffsetDateTime>, top_n: usize) -> Report {
    // Apply the time window first.
    let included: Vec<&Session> = sessions
        .iter()
        .filter(|s| match since {
            Some(cutoff) => session_time(s).map(|t| t >= cutoff).unwrap_or(false),
            None => true,
        })
        .collect();

    let mut summary = Summary {
        sessions: included.len(),
        ..Default::default()
    };

    let mut model_credits: HashMap<String, (f64, usize, usize)> = HashMap::new();
    let mut project_credits: HashMap<String, (f64, usize, usize)> = HashMap::new();
    let mut day_credits: HashMap<Date, (f64, usize)> = HashMap::new();

    for session in &included {
        let s_credits = session.credits();
        let s_turns = session.turns().len();

        summary.turns += s_turns;
        summary.credits += s_credits;
        for turn in session.turns() {
            summary.requests += turn.total_request_count.unwrap_or(0);
            summary.tool_uses += turn.builtin_tool_uses.unwrap_or(0);
            summary.input_tokens += turn.input_token_count.unwrap_or(0);
            summary.output_tokens += turn.output_token_count.unwrap_or(0);
            if let Some(d) = turn.turn_duration {
                summary.duration_secs += d.as_secs_f64();
            }
            // Bucket credits by the day the turn ended.
            if let Some(ts) = turn.end_timestamp.as_deref().and_then(parse_ts) {
                let day = ts.date();
                let entry = day_credits.entry(day).or_insert((0.0, 0));
                entry.0 += turn.credits();
            }
        }

        // By model.
        let model = session.model_id().unwrap_or("(unknown)").to_string();
        let m = model_credits.entry(model).or_insert((0.0, 0, 0));
        m.0 += s_credits;
        m.1 += 1;
        m.2 += s_turns;

        // By project.
        let project = project_label(session.cwd.as_deref());
        let p = project_credits.entry(project).or_insert((0.0, 0, 0));
        p.0 += s_credits;
        p.1 += 1;
        p.2 += s_turns;

        // Track day of session for day-session counts.
        if let Some(t) = session_time(session) {
            let day = t.date();
            day_credits.entry(day).or_insert((0.0, 0)).1 += 1;
            summary.first_day = Some(summary.first_day.map_or(day, |d| d.min(day)));
            summary.last_day = Some(summary.last_day.map_or(day, |d| d.max(day)));
        }
    }

    let total = summary.credits.max(f64::EPSILON);

    let by_model = rank_groups(model_credits, total, top_n);
    let by_project = rank_groups(project_credits, total, top_n);

    let mut by_day: Vec<DayRow> = day_credits
        .into_iter()
        .map(|(date, (credits, sessions))| DayRow {
            date,
            credits,
            sessions,
        })
        .collect();
    by_day.sort_by_key(|r| r.date);

    let recent = build_recent(&included, top_n);

    Report {
        summary,
        by_model,
        by_project,
        by_day,
        recent,
    }
}

/// Turn a label -> (credits, sessions, turns) map into a ranked, truncated list.
fn rank_groups(
    map: HashMap<String, (f64, usize, usize)>,
    total: f64,
    top_n: usize,
) -> Vec<GroupRow> {
    let mut rows: Vec<GroupRow> = map
        .into_iter()
        .map(|(label, (credits, sessions, turns))| GroupRow {
            label,
            sessions,
            turns,
            credits,
            pct: credits / total * 100.0,
        })
        .collect();
    rows.sort_by(|a, b| {
        b.credits
            .partial_cmp(&a.credits)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    rows.truncate(top_n);
    rows
}

/// Build the reverse-chronological recent feed with signed deltas.
fn build_recent(sessions: &[&Session], top_n: usize) -> Vec<RecentRow> {
    // Order by most recent activity, newest first.
    let mut ordered: Vec<(&Session, Option<OffsetDateTime>)> =
        sessions.iter().map(|s| (*s, session_time(s))).collect();
    ordered.sort_by_key(|x| std::cmp::Reverse(x.1));
    ordered.truncate(top_n);
    let mut rows = Vec::with_capacity(ordered.len());
    for (i, (session, when)) in ordered.iter().enumerate() {
        let credits = session.credits();
        // Status: worst turn status wins (Error > Cancelled > Ok).
        let status = session
            .turns()
            .iter()
            .map(|t| Status::from_end_reason(t.end_reason.as_deref()))
            .fold(Status::Ok, |acc, s| match (acc, s) {
                (Status::Error, _) | (_, Status::Error) => Status::Error,
                (Status::Cancelled, _) | (_, Status::Cancelled) => Status::Cancelled,
                _ => Status::Ok,
            });

        // Delta vs the next-older session in the feed.
        let delta_pct = ordered.get(i + 1).and_then(|(prev, _)| {
            let prev_credits = prev.credits();
            if prev_credits.abs() < f64::EPSILON {
                None
            } else {
                Some((credits - prev_credits) / prev_credits * 100.0)
            }
        });

        let title = session
            .title
            .clone()
            .or_else(|| session.cwd.clone())
            .unwrap_or_else(|| session.session_id.clone());

        rows.push(RecentRow {
            title,
            credits,
            status,
            delta_pct,
            when: *when,
        });
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Session;

    /// Build a session with a given model, cwd, and a list of
    /// (credits, end_reason, end_timestamp) turns.
    fn make_session(id: &str, model: &str, cwd: &str, turns: &[(f64, &str, &str)]) -> Session {
        let turns_json: Vec<String> = turns
            .iter()
            .map(|(c, reason, ts)| {
                format!(
                    r#"{{"end_reason":"{reason}","end_timestamp":"{ts}",
                        "metering_usage":[{{"value":{c},"unit":"credit"}}]}}"#
                )
            })
            .collect();
        let json = format!(
            r#"{{
                "session_id":"{id}",
                "cwd":"{cwd}",
                "updated_at":"{updated}",
                "title":"t-{id}",
                "session_state":{{
                    "rts_model_state":{{"model_info":{{"model_id":"{model}"}}}},
                    "conversation_metadata":{{"user_turn_metadatas":[{turns}]}}
                }}
            }}"#,
            updated = turns.last().map(|t| t.2).unwrap_or("2026-07-01T00:00:00Z"),
            turns = turns_json.join(","),
        );
        serde_json::from_str(&json).expect("session parses")
    }

    fn fixtures() -> Vec<Session> {
        vec![
            make_session(
                "s1",
                "opus",
                "/a/proj-x",
                &[(2.0, "UserTurnEnd", "2026-07-01T10:00:00Z")],
            ),
            make_session(
                "s2",
                "opus",
                "/a/proj-x",
                &[(3.0, "Cancelled", "2026-07-02T10:00:00Z")],
            ),
            make_session(
                "s3",
                "sonnet",
                "/a/proj-y",
                &[(1.0, "Error", "2026-07-02T12:00:00Z")],
            ),
        ]
    }

    #[test]
    fn summary_totals() {
        let r = aggregate(&fixtures(), None, 10);
        assert_eq!(r.summary.sessions, 3);
        assert_eq!(r.summary.turns, 3);
        assert!((r.summary.credits - 6.0).abs() < 1e-9);
    }

    #[test]
    fn by_model_ranked_desc_with_pct() {
        let r = aggregate(&fixtures(), None, 10);
        assert_eq!(r.by_model.len(), 2);
        // opus = 5.0, sonnet = 1.0
        assert_eq!(r.by_model[0].label, "opus");
        assert!((r.by_model[0].credits - 5.0).abs() < 1e-9);
        assert!((r.by_model[0].pct - (5.0 / 6.0 * 100.0)).abs() < 1e-6);
        assert_eq!(r.by_model[1].label, "sonnet");
    }

    #[test]
    fn by_project_uses_short_labels() {
        let r = aggregate(&fixtures(), None, 10);
        let labels: Vec<&str> = r.by_project.iter().map(|g| g.label.as_str()).collect();
        assert!(labels.contains(&"a/proj-x"));
        assert!(labels.contains(&"a/proj-y"));
    }

    #[test]
    fn by_day_bucketed_and_sorted() {
        let r = aggregate(&fixtures(), None, 10);
        assert_eq!(r.by_day.len(), 2);
        assert!(r.by_day[0].date < r.by_day[1].date);
        // 2026-07-02 has 3.0 + 1.0 = 4.0 credits.
        let last = r.by_day.last().unwrap();
        assert!((last.credits - 4.0).abs() < 1e-9);
    }

    #[test]
    fn top_n_truncates_breakdowns() {
        let r = aggregate(&fixtures(), None, 1);
        assert_eq!(r.by_model.len(), 1);
        assert_eq!(r.recent.len(), 1);
    }

    #[test]
    fn recent_is_reverse_chronological() {
        let r = aggregate(&fixtures(), None, 10);
        // Newest activity first: s2/s3 are on 07-02, s1 on 07-01.
        assert!(r.recent[0].when >= r.recent[1].when);
        assert!(r.recent[1].when >= r.recent[2].when);
    }

    #[test]
    fn status_derived_from_worst_turn() {
        let s = make_session(
            "mix",
            "opus",
            "/a/b",
            &[
                (1.0, "UserTurnEnd", "2026-07-01T10:00:00Z"),
                (1.0, "Error", "2026-07-01T10:05:00Z"),
            ],
        );
        let r = aggregate(&[s], None, 10);
        assert_eq!(r.recent[0].status, Status::Error);
    }

    #[test]
    fn since_filter_excludes_old_sessions() {
        let cutoff = parse_ts("2026-07-02T00:00:00Z").unwrap();
        let r = aggregate(&fixtures(), Some(cutoff), 10);
        // Only s2 and s3 are on/after the cutoff.
        assert_eq!(r.summary.sessions, 2);
    }
}
