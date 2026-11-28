//! Machine-readable success output and terminal-aware format selection.

use clap::ValueEnum;
use serde::Serialize;
use std::io::{self, IsTerminal};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(in crate::cli) enum OutputFormat {
    Auto,
    Json,
    PrettyJson,
    Human,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::cli) enum ResolvedOutputFormat {
    Json,
    PrettyJson,
    Human,
}

impl OutputFormat {
    pub(in crate::cli) fn resolve(self) -> ResolvedOutputFormat {
        self.resolve_with_terminal(io::stdout().is_terminal())
    }

    fn resolve_with_terminal(self, is_terminal: bool) -> ResolvedOutputFormat {
        match self {
            Self::Auto if is_terminal => ResolvedOutputFormat::Human,
            Self::Auto | Self::Json => ResolvedOutputFormat::Json,
            Self::PrettyJson => ResolvedOutputFormat::PrettyJson,
            Self::Human => ResolvedOutputFormat::Human,
        }
    }
}

#[derive(Debug, Serialize)]
pub(in crate::cli) struct SuccessEnvelope<T> {
    ok: bool,
    data: T,
}

impl<T> SuccessEnvelope<T> {
    pub(in crate::cli) fn new(data: T) -> Self {
        Self { ok: true, data }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_auto_and_respects_explicit_formats_on_terminal_and_pipe() {
        for (name, format, tty, pipe) in [
            (
                "auto",
                OutputFormat::Auto,
                ResolvedOutputFormat::Human,
                ResolvedOutputFormat::Json,
            ),
            (
                "json",
                OutputFormat::Json,
                ResolvedOutputFormat::Json,
                ResolvedOutputFormat::Json,
            ),
            (
                "pretty-json",
                OutputFormat::PrettyJson,
                ResolvedOutputFormat::PrettyJson,
                ResolvedOutputFormat::PrettyJson,
            ),
            (
                "human",
                OutputFormat::Human,
                ResolvedOutputFormat::Human,
                ResolvedOutputFormat::Human,
            ),
        ] {
            assert_eq!(OutputFormat::from_str(name, false).unwrap(), format);
            assert_eq!(format.resolve_with_terminal(true), tty);
            assert_eq!(format.resolve_with_terminal(false), pipe);
            assert_eq!(
                format.resolve(),
                format.resolve_with_terminal(io::stdout().is_terminal())
            );
        }
        assert!(OutputFormat::from_str("pretty_json", false).is_err());
    }

    #[test]
    fn success_envelope_preserves_data_and_propagates_serialization_errors() {
        let data = serde_json::json!({"message": "\\\"\n猫", "count": 0});
        let envelope = SuccessEnvelope::new(&data);
        let json = serde_json::to_string(&envelope).unwrap();
        let pretty = serde_json::to_string_pretty(&envelope).unwrap();
        assert!(!json.contains('\n'));
        assert!(pretty.contains("\n  \"ok\": true,"));
        for text in [json, pretty] {
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&text).unwrap(),
                serde_json::json!({"ok": true, "data": data})
            );
        }
        assert_eq!(
            serde_json::to_value(SuccessEnvelope::new(())).unwrap(),
            serde_json::json!({"ok": true, "data": null})
        );

        struct Unserializable;
        impl Serialize for Unserializable {
            fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
                Err(serde::ser::Error::custom("fixture serialization failure"))
            }
        }
        assert!(serde_json::to_string(&SuccessEnvelope::new(Unserializable)).is_err());
    }
}
