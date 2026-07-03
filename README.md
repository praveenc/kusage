# kusage

A local, fast CLI that displays [Kiro CLI](https://kiro.dev) coding-agent usage metrics with a compact, professional dashboard.

Inspired by [ccusage](https://github.com/ccusage/ccusage) for the concept, and by `rtk`'s history output for the look.
It reads Kiro CLI's local session data and shows how much you're using it: sessions, turns, requests, tool uses, and credits, broken down by model, project, and day.
Everything runs locally: no network calls, no telemetry.

## Example

```
────────────────────────────────────────────────────────────────────────
kusage  Kiro CLI usage
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

Status glyphs in the recent feed: `✓` completed, `~` cancelled/interrupted, `✗` error.

## Install

From source (requires Rust 1.75+):

```bash
cargo install --path .
```

Or build a release binary:

```bash
cargo build --release
# binary at target/release/kusage
```

## Usage

```bash
kusage                 # full dashboard
kusage --since 7       # only the last 7 days
kusage --top 5         # limit breakdowns and recent feed to 5 entries
kusage --json          # machine-readable JSON for scripting
kusage --plain         # no colors (also honors NO_COLOR and non-TTY output)
```

| Flag | Description | Default |
| --- | --- | --- |
| `--since <DAYS>` | Only include usage from the last N days | all history |
| `--top <N>` | Limit ranked breakdowns and the recent feed | 10 |
| `--json` | Emit JSON instead of the dashboard | off |
| `--plain` | Disable colors and styling | off |

## Where the data comes from

Kiro CLI stores each chat session as a JSON sidecar file at:

```
~/.kiro/sessions/cli/<session-uuid>.json
```

Each file records per-turn metadata: credit metering, request and tool-use counts, timing, end reason, and the model in use.
`kusage` reads these files read-only and aggregates them. It never writes back to Kiro's data.

If your Kiro home lives elsewhere, set `KIRO_DIR` (or `KIRO_HOME`) to point at it; `kusage` will look under `$KIRO_DIR/sessions/cli`.

### A note on tokens vs. credits

Kiro's usage data currently reports cost as **credits** (with a per-model rate multiplier), not raw token counts.
The token fields exist in Kiro's schema but are not populated today, so `kusage` shows `Tokens: n/a` until Kiro starts reporting them.
Credits are the headline cost metric.

## Scope

- **In:** read-only local parse, aggregation by model / project / day / session, the dashboard, a `--json` mode for scripting, and flags for time window, top-N, and plain output.
- **Out (v1):** no network calls, no telemetry, no writing back to Kiro's data, no daemon or watch mode.

## License

[MIT](LICENSE) © Praveen Chamarthi
