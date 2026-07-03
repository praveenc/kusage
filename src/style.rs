//! Terminal styling helpers.
//!
//! Emits ANSI escape codes directly (no TUI framework needed for a print-once
//! dashboard). Colors are disabled automatically when output is not a TTY, when
//! `NO_COLOR` is set, or when the caller requests plain mode. Truecolor is used
//! when the terminal advertises it via `COLORTERM`, with a graceful fallback to
//! the 16-color palette otherwise.

use std::io::IsTerminal;

/// How much color to emit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    /// No escape codes at all.
    None,
    /// Basic 16-color ANSI.
    Ansi16,
    /// 24-bit truecolor.
    True,
}

impl ColorMode {
    /// Detect the appropriate color mode for stdout.
    ///
    /// `force_plain` forces [`ColorMode::None`] regardless of the terminal.
    pub fn detect(force_plain: bool) -> ColorMode {
        if force_plain || std::env::var_os("NO_COLOR").is_some() || !std::io::stdout().is_terminal()
        {
            return ColorMode::None;
        }
        match std::env::var("COLORTERM").as_deref() {
            Ok("truecolor") | Ok("24bit") => ColorMode::True,
            _ => ColorMode::Ansi16,
        }
    }
}

/// An RGB color plus a nearest 16-color ANSI fallback code.
#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    /// ANSI SGR foreground code used when truecolor is unavailable (e.g. 32).
    pub ansi16: u8,
}

impl Color {
    const fn new(r: u8, g: u8, b: u8, ansi16: u8) -> Color {
        Color { r, g, b, ansi16 }
    }
}

/// A small calm palette tuned for the dashboard.
pub struct Palette;

impl Palette {
    pub const GREEN: Color = Color::new(126, 200, 128, 32);
    pub const YELLOW: Color = Color::new(224, 192, 96, 33);
    pub const RED: Color = Color::new(224, 108, 108, 31);
    pub const BLUE: Color = Color::new(122, 162, 224, 34);
    pub const CYAN: Color = Color::new(120, 200, 200, 36);
}

/// Wrap `text` in the escape codes for `color` under the given `mode`.
pub fn paint(mode: ColorMode, color: Color, text: &str) -> String {
    match mode {
        ColorMode::None => text.to_string(),
        ColorMode::Ansi16 => format!("\x1b[{}m{}\x1b[0m", color.ansi16, text),
        ColorMode::True => format!(
            "\x1b[38;2;{};{};{}m{}\x1b[0m",
            color.r, color.g, color.b, text
        ),
    }
}

/// Bold `text` (falls back to plain when color is disabled).
pub fn bold(mode: ColorMode, text: &str) -> String {
    match mode {
        ColorMode::None => text.to_string(),
        _ => format!("\x1b[1m{}\x1b[0m", text),
    }
}

/// Dim `text` (falls back to plain when color is disabled).
pub fn dim(mode: ColorMode, text: &str) -> String {
    match mode {
        ColorMode::None => text.to_string(),
        _ => format!("\x1b[2m{}\x1b[0m", text),
    }
}

/// Choose a color for a percentage using calm thresholds.
pub fn pct_color(pct: f64) -> Color {
    if pct >= 50.0 {
        Palette::RED
    } else if pct >= 20.0 {
        Palette::YELLOW
    } else {
        Palette::GREEN
    }
}
