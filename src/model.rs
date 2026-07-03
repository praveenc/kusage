//! Data model for Kiro CLI session metadata.
//!
//! Kiro CLI stores each chat session as a sidecar JSON file at
//! `~/.kiro/sessions/cli/<uuid>.json`. Each file holds session-level metadata
//! plus a list of "user turns", each carrying credit metering, request counts,
//! tool-use counts, timing, and (currently always zero) token counts.
//!
//! The structs below are intentionally lenient: every field Kiro might rename
//! or omit across versions is optional, so a schema drift degrades gracefully
//! instead of failing the whole parse. Some fields are parsed for completeness
//! and future use even if the current report does not surface them.
#![allow(dead_code)]

use serde::Deserialize;

/// A single Kiro CLI session, parsed from one `<uuid>.json` sidecar file.
#[derive(Debug, Clone, Deserialize)]
pub struct Session {
    /// Session UUID (also the file stem).
    pub session_id: String,
    /// Working directory the session was started in.
    #[serde(default)]
    pub cwd: Option<String>,
    /// ISO-8601 creation timestamp.
    #[serde(default)]
    pub created_at: Option<String>,
    /// ISO-8601 last-updated timestamp.
    #[serde(default)]
    pub updated_at: Option<String>,
    /// Human-readable title (usually the first prompt, truncated).
    #[serde(default)]
    pub title: Option<String>,
    /// Why the session was created (e.g. "user", "subagent").
    #[serde(default)]
    pub session_created_reason: Option<String>,
    /// Nested session state, holding per-turn metadata and model info.
    #[serde(default)]
    pub session_state: Option<SessionState>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SessionState {
    #[serde(default)]
    pub agent_name: Option<String>,
    #[serde(default)]
    pub conversation_metadata: Option<ConversationMetadata>,
    #[serde(default)]
    pub rts_model_state: Option<RtsModelState>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RtsModelState {
    #[serde(default)]
    pub model_info: Option<ModelInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelInfo {
    #[serde(default)]
    pub model_id: Option<String>,
    #[serde(default)]
    pub model_name: Option<String>,
    #[serde(default)]
    pub rate_multiplier: Option<f64>,
    #[serde(default)]
    pub context_window_tokens: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConversationMetadata {
    /// One entry per user turn (a user prompt and the agent work it drove).
    #[serde(default)]
    pub user_turn_metadatas: Vec<UserTurnMetadata>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserTurnMetadata {
    /// Number of model requests made during this turn.
    #[serde(default)]
    pub total_request_count: Option<u64>,
    /// Number of built-in tool invocations during this turn.
    #[serde(default)]
    pub builtin_tool_uses: Option<u64>,
    /// How the turn ended: "UserTurnEnd", "Cancelled", "Error", "ToolUseRejected".
    #[serde(default)]
    pub end_reason: Option<String>,
    /// ISO-8601 timestamp when the turn ended.
    #[serde(default)]
    pub end_timestamp: Option<String>,
    /// Input tokens for the turn. Present in schema but currently always 0.
    #[serde(default)]
    pub input_token_count: Option<u64>,
    /// Output tokens for the turn. Present in schema but currently always 0.
    #[serde(default)]
    pub output_token_count: Option<u64>,
    /// Fraction of the context window used, as a percentage.
    #[serde(default)]
    pub context_usage_percentage: Option<f64>,
    /// Wall-clock duration of the turn.
    #[serde(default)]
    pub turn_duration: Option<SecsNanos>,
    /// Per-request credit metering for the turn. Sum these for turn cost.
    #[serde(default)]
    pub metering_usage: Vec<MeteringEntry>,
}

/// A duration expressed as whole seconds plus nanoseconds, as Kiro serializes it.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct SecsNanos {
    #[serde(default)]
    pub secs: u64,
    #[serde(default)]
    pub nanos: u64,
}

impl SecsNanos {
    /// Total duration in seconds as a float.
    pub fn as_secs_f64(&self) -> f64 {
        self.secs as f64 + (self.nanos as f64 / 1_000_000_000.0)
    }
}

/// One metering line item. Kiro emits `unit: "credit"` today.
#[derive(Debug, Clone, Deserialize)]
pub struct MeteringEntry {
    #[serde(default)]
    pub value: f64,
    #[serde(default)]
    pub unit: Option<String>,
}

impl UserTurnMetadata {
    /// Total credits consumed by this turn (sum of credit-unit metering entries).
    pub fn credits(&self) -> f64 {
        self.metering_usage
            .iter()
            .filter(|m| m.unit.as_deref() == Some("credit"))
            .map(|m| m.value)
            .sum()
    }
}

impl Session {
    /// The model id currently associated with the session, if known.
    pub fn model_id(&self) -> Option<&str> {
        self.session_state
            .as_ref()?
            .rts_model_state
            .as_ref()?
            .model_info
            .as_ref()?
            .model_id
            .as_deref()
    }

    /// The session's agent name, if known.
    pub fn agent_name(&self) -> Option<&str> {
        self.session_state.as_ref()?.agent_name.as_deref()
    }

    /// All user turns for this session (empty if metadata is absent).
    pub fn turns(&self) -> &[UserTurnMetadata] {
        self.session_state
            .as_ref()
            .and_then(|s| s.conversation_metadata.as_ref())
            .map(|c| c.user_turn_metadatas.as_slice())
            .unwrap_or(&[])
    }

    /// Total credits consumed across all turns in the session.
    pub fn credits(&self) -> f64 {
        self.turns().iter().map(|t| t.credits()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal but realistic session sidecar, mirroring Kiro's schema.
    const SAMPLE: &str = r#"{
        "session_id": "abc-123",
        "cwd": "/Users/me/projects/demo",
        "created_at": "2026-07-01T10:00:00Z",
        "updated_at": "2026-07-01T10:30:00Z",
        "title": "build a thing",
        "session_created_reason": "user",
        "session_state": {
            "agent_name": "kiro_default",
            "rts_model_state": {
                "model_info": {
                    "model_id": "claude-opus-4.8",
                    "model_name": "Claude Opus 4.8",
                    "rate_multiplier": 2.2,
                    "context_window_tokens": 1000000
                }
            },
            "conversation_metadata": {
                "user_turn_metadatas": [
                    {
                        "total_request_count": 3,
                        "builtin_tool_uses": 5,
                        "end_reason": "UserTurnEnd",
                        "end_timestamp": "2026-07-01T10:10:00Z",
                        "input_token_count": 0,
                        "output_token_count": 0,
                        "turn_duration": {"secs": 12, "nanos": 500000000},
                        "metering_usage": [
                            {"value": 1.5, "unit": "credit"},
                            {"value": 0.5, "unit": "credit"}
                        ]
                    },
                    {
                        "total_request_count": 2,
                        "builtin_tool_uses": 1,
                        "end_reason": "Cancelled",
                        "end_timestamp": "2026-07-01T10:20:00Z",
                        "metering_usage": [
                            {"value": 1.0, "unit": "credit"}
                        ]
                    }
                ]
            }
        }
    }"#;

    fn sample() -> Session {
        serde_json::from_str(SAMPLE).expect("sample parses")
    }

    #[test]
    fn parses_core_fields() {
        let s = sample();
        assert_eq!(s.session_id, "abc-123");
        assert_eq!(s.cwd.as_deref(), Some("/Users/me/projects/demo"));
        assert_eq!(s.model_id(), Some("claude-opus-4.8"));
        assert_eq!(s.agent_name(), Some("kiro_default"));
        assert_eq!(s.turns().len(), 2);
    }

    #[test]
    fn sums_credits_across_turns() {
        let s = sample();
        // 1.5 + 0.5 + 1.0
        assert!((s.credits() - 3.0).abs() < 1e-9);
    }

    #[test]
    fn per_turn_credits_ignore_non_credit_units() {
        let turn = sample().turns()[0].clone();
        assert!((turn.credits() - 2.0).abs() < 1e-9);
    }

    #[test]
    fn tolerates_missing_optional_fields() {
        // No session_state at all.
        let minimal = r#"{"session_id": "x"}"#;
        let s: Session = serde_json::from_str(minimal).expect("minimal parses");
        assert_eq!(s.turns().len(), 0);
        assert_eq!(s.credits(), 0.0);
        assert_eq!(s.model_id(), None);
    }

    #[test]
    fn secs_nanos_to_float() {
        let d = SecsNanos {
            secs: 12,
            nanos: 500_000_000,
        };
        assert!((d.as_secs_f64() - 12.5).abs() < 1e-9);
    }
}
