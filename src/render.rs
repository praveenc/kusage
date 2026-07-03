//! Print-once dashboard renderer.
//!
//! Renders the aggregated [`Report`] as a compact, rtk-inspired dashboard:
//! an aligned summary header with a utilization bar, numbered "By X" tables
//! with color-coded percentages and right-aligned impact bars, a per-day
//! mini-bar column, and a reverse-chronological recent feed with status glyphs
//! and signed deltas. Section rules separate the blocks.

use crate::aggregate::{DayRow, GroupRow, RecentRow, Report};
use crate::style::{bold, dim, paint, pct_color, Color, ColorMode, Palette};

/// Total inner width of the dashboard content.
const WIDTH: usize = 72;
/// Width of the impact/utilization bars.
const BAR_WIDTH: usize = 16;

const FILLED: char = '█';
const EMPTY: char = '░';

/// Compact ASCII wordmark shown above the dashboard (figlet "small" font).
/// Kept understated to match the calm dashboard feel; suppressed in plain mode.
const BANNER: &[&str] = &[
    r" _",
    r"| |___  _ ___ __ _ __ _ ___",
    r"| / / || (_-</ _` / _` / -_)",
    r"|_\_\\_,_/__/\__,_\__, \___|",
    r"                  |___/",
];

/// Render the full report to a `String`.
pub fn render(report: &Report, mode: ColorMode) -> String {
    let mut out = String::new();
    render_banner(&mut out, mode);
    render_header(&mut out, report, mode);
    render_by_group(&mut out, "By Model", &report.by_model, mode);
    render_by_group(&mut out, "By Project", &report.by_project, mode);
    render_by_day(&mut out, &report.by_day, mode);
    render_recent(&mut out, &report.recent, mode);
    out
}

/// Render the ASCII wordmark. Skipped in plain/no-color mode so piped or
/// `--plain` output stays clean and machine-friendly.
fn render_banner(out: &mut String, mode: ColorMode) {
    if mode == ColorMode::None {
        return;
    }
    for line in BANNER {
        out.push_str(&paint(mode, Palette::CYAN, line));
        out.push('\n');
    }
    out.push('\n');
}

/// A horizontal rule spanning the dashboard width.
fn rule(out: &mut String, mode: ColorMode) {
    out.push_str(&dim(mode, &"─".repeat(WIDTH)));
    out.push('\n');
}

/// A section title line, e.g. `── By Model ──────────`.
fn section(out: &mut String, title: &str, mode: ColorMode) {
    let label = format!("── {title} ");
    let pad = WIDTH.saturating_sub(label.chars().count());
    let line = format!("{label}{}", "─".repeat(pad));
    out.push_str(&dim(mode, &line));
    out.push('\n');
}

/// Build a block bar of `width` cells representing `frac` (0.0..=1.0).
fn bar(frac: f64, width: usize, color: Color, mode: ColorMode) -> String {
    let frac = frac.clamp(0.0, 1.0);
    let filled = (frac * width as f64).round() as usize;
    let filled = filled.min(width);
    let bar: String = std::iter::repeat(FILLED)
        .take(filled)
        .chain(std::iter::repeat(EMPTY).take(width - filled))
        .collect();
    paint(mode, color, &bar)
}

/// Format credits compactly (e.g. `1.2k`, `342`, `4.3`).
fn fmt_credits(c: f64) -> String {
    // Guard against tiny negatives/`-0.0` from floating-point summation.
    let c = if c.abs() < 0.05 { 0.0 } else { c };
    if c >= 1000.0 {
        format!("{:.1}k", c / 1000.0)
    } else if c >= 100.0 {
        format!("{:.0}", c)
    } else {
        format!("{:.1}", c)
    }
}

fn render_header(out: &mut String, report: &Report, mode: ColorMode) {
    let s = &report.summary;

    // When the banner is suppressed (plain mode) fall back to a text title so
    // the output is still self-identifying.
    if mode == ColorMode::None {
        out.push_str("kusage  Kiro CLI usage\n");
    } else {
        out.push_str(&dim(mode, "Kiro CLI usage"));
        out.push('\n');
    }

    if let (Some(first), Some(last)) = (s.first_day, s.last_day) {
        let range = format!("{}  →  {}", first, last);
        out.push_str(&dim(mode, &range));
        out.push('\n');
    }
    out.push('\n');

    // Two aligned metric rows.
    let line1 = format!(
        "  {:<10}{:>8}    {:<10}{:>8}",
        "Sessions", s.sessions, "Turns", s.turns,
    );
    let line2 = format!(
        "  {:<10}{:>8}    {:<10}{:>8}",
        "Requests", s.requests, "Tool uses", s.tool_uses,
    );
    out.push_str(&line1);
    out.push('\n');
    out.push_str(&line2);
    out.push('\n');

    // Credits headline with a utilization bar (relative to the busiest day).
    let peak_day = report
        .by_day
        .iter()
        .map(|d| d.credits)
        .fold(0.0_f64, f64::max)
        .max(f64::EPSILON);
    let today = report.by_day.last().map(|d| d.credits).unwrap_or(0.0);
    let frac = today / peak_day;

    let credits_label = format!("  {:<10}{:>8}", "Credits", fmt_credits(s.credits));
    out.push_str(&bold(mode, &credits_label));
    out.push('\n');

    let util = format!(
        "  {:<10}{} {}",
        "Latest",
        bar(frac, BAR_WIDTH, Palette::CYAN, mode),
        dim(mode, &format!("{} of peak day", fmt_credits(today))),
    );
    out.push_str(&util);
    out.push('\n');

    // Tokens line, flagged when Kiro is not populating them.
    let tokens = if s.input_tokens == 0 && s.output_tokens == 0 {
        dim(mode, "  Tokens     n/a (not reported by Kiro)")
    } else {
        format!(
            "  {:<10}{} in / {} out",
            "Tokens", s.input_tokens, s.output_tokens
        )
    };
    out.push_str(&tokens);
    out.push('\n');
    out.push('\n');
}

fn render_by_group(out: &mut String, title: &str, rows: &[GroupRow], mode: ColorMode) {
    section(out, title, mode);
    if rows.is_empty() {
        out.push_str(&dim(mode, "  (no data)\n\n"));
        return;
    }
    let max = rows
        .iter()
        .map(|r| r.credits)
        .fold(0.0_f64, f64::max)
        .max(f64::EPSILON);
    for (i, row) in rows.iter().enumerate() {
        let label = truncate(&row.label, 22);
        let pct = format!("{:>5.1}%", row.pct);
        let pct = paint(mode, pct_color(row.pct), &pct);
        let impact = bar(row.credits / max, BAR_WIDTH, Palette::BLUE, mode);
        let line = format!(
            "  {:>2}. {:<22} {:>7} {} {}",
            i + 1,
            label,
            fmt_credits(row.credits),
            pct,
            impact,
        );
        out.push_str(&line);
        out.push('\n');
    }
    out.push('\n');
}

fn render_by_day(out: &mut String, rows: &[DayRow], mode: ColorMode) {
    section(out, "By Day", mode);
    if rows.is_empty() {
        out.push_str(&dim(mode, "  (no data)\n\n"));
        return;
    }
    let max = rows
        .iter()
        .map(|r| r.credits)
        .fold(0.0_f64, f64::max)
        .max(f64::EPSILON);
    for row in rows {
        let impact = bar(row.credits / max, BAR_WIDTH, Palette::GREEN, mode);
        let line = format!(
            "  {}  {} {:>7}  {}",
            row.date,
            impact,
            fmt_credits(row.credits),
            dim(mode, &format!("{} sess", row.sessions)),
        );
        out.push_str(&line);
        out.push('\n');
    }
    out.push('\n');
}

fn render_recent(out: &mut String, rows: &[RecentRow], mode: ColorMode) {
    section(out, "Recent Sessions", mode);
    if rows.is_empty() {
        out.push_str(&dim(mode, "  (no data)\n\n"));
        return;
    }
    for (i, row) in rows.iter().enumerate() {
        let glyph_color = match row.status {
            crate::aggregate::Status::Ok => Palette::GREEN,
            crate::aggregate::Status::Cancelled => Palette::YELLOW,
            crate::aggregate::Status::Error => Palette::RED,
        };
        let glyph = paint(mode, glyph_color, &row.status.glyph().to_string());
        let title = truncate(&row.title, 38);
        let credits = format!("{:>7}", fmt_credits(row.credits));

        let delta = match row.delta_pct {
            Some(d) => {
                let sign = if d >= 0.0 { "+" } else { "" };
                let color = if d >= 0.0 {
                    Palette::RED
                } else {
                    Palette::GREEN
                };
                paint(mode, color, &format!("{sign}{d:.0}%"))
            }
            None => dim(mode, "  ·"),
        };

        let line = format!(
            "  {:>2}. {} {:<38} {} {:>6}",
            i + 1,
            glyph,
            title,
            credits,
            delta
        );
        out.push_str(&line);
        out.push('\n');
    }
    rule(out, mode);
}

/// Truncate a string to `max` display chars, adding an ellipsis when cut.
fn truncate(s: &str, max: usize) -> String {
    let s = s.replace(['\n', '\r'], " ");
    if s.chars().count() <= max {
        s
    } else {
        let taken: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{taken}…")
    }
}
