<div align="center">

# kusage

**See how much you're using Kiro CLI, right from your terminal.**

A local, fast Rust CLI that turns [Kiro CLI](https://kiro.dev)'s session data into a compact, professional usage dashboard: sessions, turns, requests, tool uses, and credits, broken down by model, project, and day.

> [!NOTE]
> `kusage` targets **Kiro CLI v2** (the current release). Kiro CLI v3 is on the way; support for its data format will follow once it lands.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)
[![Local only](https://img.shields.io/badge/network-none-brightgreen.svg)](#privacy)

<img src="docs/assets/kusage-dashboard.png" alt="kusage dashboard showing summary, by-model, by-project, by-day, and recent sessions" width="600">

</div>

---

## Why kusage

Kiro CLI tracks your usage locally, but there's no easy way to *see* it. `kusage` reads that local data and shows it as a calm, information-dense dashboard, so you can answer questions like: how many credits did I burn this week, which model and project drove most of my usage, and did my recent sessions succeed?

Everything runs **locally**: no network calls, no telemetry, no writing back to Kiro's data.

## Install

`kusage` is a Rust CLI, so you need the Rust toolchain first. If you don't have it, install it from [rustup.rs](https://rustup.rs):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Then install `kusage`:

```bash
# From crates.io (once published)
cargo install kusage

# Or from source
git clone https://github.com/praveenc/kusage
cd kusage
cargo install --path .
```

Either way you get a `kusage` binary on your `PATH`. Prefer not to install? Build a release binary and run it in place:

```bash
git clone https://github.com/praveenc/kusage
cd kusage
cargo build --release
./target/release/kusage
```

To update a source install later, pull and reinstall:

```bash
git pull
cargo install --path . --force
```

## Quick start

Just run it:

```bash
kusage
```

That prints the full dashboard for all of your Kiro CLI history. From there, narrow it down.

## Everyday usage

`kusage` is a single command with a few flags. Combine them freely.

**Focus on a time window.** Only look at the last N days:

```bash
kusage --since 7         # this week
kusage --since 30        # this month
```

**Trim the tables.** Show only the top rows in each ranked section (By Model, By Project, Recent):

```bash
kusage --top 5           # top 5 rows per section
kusage --since 30 --top 8
```

**Plain text.** Drop colors and the banner (useful for logs, notes, or copy-paste):

```bash
kusage --plain
```

Colors and the banner are also disabled automatically when output isn't a terminal (piped to a file or another command) or when `NO_COLOR` is set, so you never get escape codes where you don't want them.

### All options

```
kusage [OPTIONS]
```

| Flag | Description | Default |
| --- | --- | --- |
| `--since <DAYS>` | Only include usage from the last N days | all history |
| `--top <N>` | Limit ranked breakdowns and the recent feed to N rows | `10` |
| `--json` | Emit machine-readable JSON instead of the dashboard | off |
| `--plain` | Disable colors and the banner (plain text) | off |
| `-h`, `--help` | Print help | |
| `-V`, `--version` | Print version | |

### Scripting with `--json`

`--json` prints a stable, structured report you can pipe into other tools:

```bash
# Total credits over the last 30 days
kusage --since 30 --json | jq '.summary.credits'

# Your top model by credits
kusage --json | jq -r '.by_model[0].label'

# Credits per day as a TSV
kusage --json | jq -r '.by_day[] | [.date, .credits] | @tsv'
```

### Reading the dashboard

The dashboard has five sections, top to bottom:

| Section | What it shows |
| --- | --- |
| **Summary** | Total sessions, turns, requests, tool uses, and credits, with a bar comparing your latest day to your busiest. |
| **By Model** | Which models you use most, ranked by credits, with an impact bar and a color-coded share. |
| **By Project** | Same ranking, grouped by the working directory each session ran in. |
| **By Day** | A day-by-day mini-bar timeline of credit usage. |
| **Recent Sessions** | Your latest sessions in reverse-chronological order, with a status glyph and the change in credits versus the previous session. |

Status glyphs in the recent feed:

| Glyph | Meaning |
| --- | --- |
| `✓` | Completed normally |
| `~` | Cancelled or interrupted |
| `✗` | Ended in an error |

<details>
<summary>Example output (plain text)</summary>

```
kusage  Kiro CLI (v2) usage
2026-06-29  →  2026-07-03

  Sessions        12    Turns          178
  Requests       917    Tool uses      812
  Credits       1.7k
  Latest    ████░░░░░░░░░░░░ 223 of peak day
  Tokens     n/a (not reported by Kiro)

── By Model ────────────────────────────────────────────────────────────
   1. claude-opus-4.8           1.7k  98.1% ████████████████
   2. claude-opus-4.6           31.7   1.8% ░░░░░░░░░░░░░░░░
   3. claude-sonnet-4.6          0.9   0.1% ░░░░░░░░░░░░░░░░

── By Project ──────────────────────────────────────────────────────────
   1. work/service-api          1.7k  95.8% ████████████████
   2. home/dotfiles             73.1   4.2% █░░░░░░░░░░░░░░░

── By Day ──────────────────────────────────────────────────────────────
  2026-06-29  ██░░░░░░░░░░░░░░    92.5  2 sess
  2026-06-30  ░░░░░░░░░░░░░░░░     5.0  2 sess
  2026-07-01  ████████████████     900  1 sess
  2026-07-02  █████████░░░░░░░     521  5 sess
  2026-07-03  ████░░░░░░░░░░░░     223  2 sess

── Recent Sessions ─────────────────────────────────────────────────────
   1. ✓ add pagination to the users endpoint      21.4   +12%
   2. ~ refactor the auth middleware               9.3   -57%
   3. ✗ migrate the config loader                  1.2    -87%
────────────────────────────────────────────────────────────────────────
```

</details>

## Contributing

Issues and pull requests are welcome. To work on `kusage`:

```bash
cargo build            # build
cargo test             # run the test suite
cargo clippy           # lint
cargo fmt              # format
```

---

## How it works

<details open>
<summary><strong>Where the data comes from</strong></summary>

Kiro CLI stores each chat session as a JSON file at:

```
~/.kiro/sessions/cli/<session-uuid>.json
```

Each file records per-turn metadata: credit metering, request and tool-use counts, timing, how the turn ended, and the model in use. `kusage` reads these files **read-only** and aggregates them. It never modifies Kiro's data.

If your Kiro home lives somewhere else, point `kusage` at it:

```bash
export KIRO_DIR=/path/to/your/.kiro   # kusage reads $KIRO_DIR/sessions/cli
kusage
```

</details>

<details>
<summary><strong>Credits, not tokens</strong></summary>

Kiro reports cost as **credits** (each model has a rate multiplier), not raw token counts. The token fields exist in Kiro's data but aren't populated yet, so `kusage` shows `Tokens: n/a` until Kiro starts reporting them. Credits are the headline cost metric everywhere in the dashboard.

</details>

<details>
<summary><strong>Privacy</strong></summary>

`kusage` is entirely local:

- No network calls, ever.
- No telemetry or analytics.
- Read-only: it never writes to Kiro's session data.

The only files it reads are your local Kiro session JSON files.

</details>

<details>
<summary><strong>Scope</strong></summary>

- **In:** read-only local parsing, aggregation by model / project / day / session, the dashboard, a `--json` mode, and flags for time window, top-N, and plain output.
- **Not yet (v1):** no network features, no telemetry, no watch/live mode, no export beyond `--json`.

Inspired by [ccusage](https://github.com/ccusage/ccusage) for the concept and by `rtk`'s history view for the look.

</details>

## License

[MIT](LICENSE) © Praveen Chamarthi
