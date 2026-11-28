//! Machine-readable envelopes and terminal-aware output selection for
//! operational CLI commands.
#![expect(
    dead_code,
    reason = "plumbing lands first; consumers arrive with the config, doctor, \
              and database-migrate command sub-issues"
)]

use crate::errors::AsterError;
use clap::ValueEnum;
use serde::Serialize;
use std::io::{self, IsTerminal};

/// User-selectable output format for operational commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable when stdout is a terminal, JSON otherwise.
    Auto,
    Json,
    PrettyJson,
    Human,
}

/// Output format after terminal detection has been applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedOutputFormat {
    Json,
    PrettyJson,
    Human,
}

impl OutputFormat {
    /// Resolves [`OutputFormat::Auto`] against stdout; explicit selections
    /// pass through unchanged.
    pub fn resolve(self) -> ResolvedOutputFormat {
        match self {
            Self::Auto => {
                if io::stdout().is_terminal() {
                    ResolvedOutputFormat::Human
                } else {
                    ResolvedOutputFormat::Json
                }
            }
            Self::Json => ResolvedOutputFormat::Json,
            Self::PrettyJson => ResolvedOutputFormat::PrettyJson,
            Self::Human => ResolvedOutputFormat::Human,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SuccessEnvelope<T> {
    pub ok: bool,
    pub data: T,
}

#[derive(Debug, Serialize)]
pub struct ErrorEnvelope {
    pub ok: bool,
    pub error: ErrorPayload,
}

#[derive(Debug, Serialize)]
pub struct ErrorPayload {
    pub code: &'static str,
    pub message: String,
}

/// Renders a success payload as JSON inside the shared envelope.
pub fn render_success_envelope<T>(data: &T, pretty: bool) -> String
where
    T: Serialize,
{
    let envelope = SuccessEnvelope { ok: true, data };
    if pretty {
        serde_json::to_string_pretty(&envelope)
            .unwrap_or_else(|error| render_serialization_error(error, true))
    } else {
        serde_json::to_string(&envelope)
            .unwrap_or_else(|error| render_serialization_error(error, false))
    }
}

/// Renders an [`AsterError`] as JSON inside the shared error envelope.
pub fn render_error_json(error: &AsterError, pretty: bool) -> String {
    let envelope = ErrorEnvelope {
        ok: false,
        error: ErrorPayload {
            code: error.code(),
            message: error.message().to_string(),
        },
    };
    if pretty {
        serde_json::to_string_pretty(&envelope)
            .unwrap_or_else(|serialization_error| {
                render_serialization_error(serialization_error, true)
            })
    } else {
        serde_json::to_string(&envelope)
            .unwrap_or_else(|serialization_error| {
                render_serialization_error(serialization_error, false)
            })
    }
}

fn render_serialization_error(error: serde_json::Error, pretty: bool) -> String {
    let message = format!("failed to serialize CLI envelope: {error}");
    let fallback = ErrorEnvelope {
        ok: false,
        error: ErrorPayload {
            code: "internal_error",
            message,
        },
    };
    if pretty {
        serde_json::to_string_pretty(&fallback).unwrap_or_default()
    } else {
        serde_json::to_string(&fallback).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Serialize, PartialEq)]
    struct SampleData {
        key: String,
    }

    #[test]
    fn explicit_formats_pass_through_resolution() {
        assert_eq!(OutputFormat::Json.resolve(), ResolvedOutputFormat::Json);
        assert_eq!(
            OutputFormat::PrettyJson.resolve(),
            ResolvedOutputFormat::PrettyJson
        );
        assert_eq!(OutputFormat::Human.resolve(), ResolvedOutputFormat::Human);
    }

    #[test]
    fn success_envelope_wraps_data_with_ok_flag() {
        let rendered = render_success_envelope(&SampleData {
            key: "value".to_string(),
        }, false);
        let parsed: serde_json::Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(parsed["ok"], serde_json::Value::Bool(true));
        assert_eq!(parsed["data"]["key"], "value");
    }

    #[test]
    fn error_envelope_carries_code_and_message_without_panicking() {
        let error = AsterError::validation_error("something was rejected");
        let rendered = render_error_json(&error, false);
        let parsed: serde_json::Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(parsed["ok"], serde_json::Value::Bool(false));
        assert_eq!(
            parsed["error"]["code"],
            serde_json::Value::String(error.code().to_string())
        );
        assert!(parsed["error"]["message"].as_str().is_some());
    }
}
