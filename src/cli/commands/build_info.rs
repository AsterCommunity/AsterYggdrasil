//! Build identity presentation for the version command.

fn dirty_suffix(dirty: Option<bool>) -> &'static str {
    match dirty {
        Some(true) => " (dirty)",
        Some(false) => "",
        None => " (working tree unknown)",
    }
}

fn pretty_json<T: serde::Serialize>(value: &T) -> crate::errors::Result<String> {
    serde_json::to_string_pretty(value).map_err(|_| {
        crate::errors::AsterError::internal_error("cannot serialize build information")
    })
}

pub(in crate::cli) fn render_build_info(json: bool) -> crate::errors::Result<String> {
    #[derive(serde::Serialize)]
    struct BuildInfo {
        product: &'static str,
        version: &'static str,
        revision: &'static str,
        dirty: Option<bool>,
        target: &'static str,
        profile: &'static str,
        features: Vec<&'static str>,
        build_time: &'static str,
    }

    let info = BuildInfo {
        product: "AsterYggdrasil",
        version: env!("CARGO_PKG_VERSION"),
        revision: env!("ASTER_BUILD_REVISION"),
        dirty: match env!("ASTER_BUILD_DIRTY") {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        },
        target: env!("ASTER_BUILD_TARGET"),
        profile: env!("ASTER_BUILD_PROFILE"),
        features: env!("ASTER_BUILD_FEATURES")
            .split(',')
            .filter(|feature| !feature.is_empty())
            .collect(),
        build_time: env!("ASTER_BUILD_TIME"),
    };
    if json {
        return pretty_json(&info);
    }
    let revision = format!("{}{}", info.revision, dirty_suffix(info.dirty));
    Ok([
        format!("{} {}", info.product, info.version),
        format!("Revision:  {revision}"),
        format!("Target:    {}", info.target),
        format!("Profile:   {}", info.profile),
        format!(
            "Features:  {}",
            if info.features.is_empty() {
                "none".into()
            } else {
                info.features.join(", ")
            }
        ),
        format!("Built:     {}", info.build_time),
    ]
    .join("\n"))
}

#[cfg(test)]
mod tests {
    use super::{dirty_suffix, pretty_json};
    use serde::Serialize;

    #[test]
    fn dirty_suffix_distinguishes_clean_dirty_and_unknown_worktrees() {
        assert_eq!(dirty_suffix(Some(true)), " (dirty)");
        assert_eq!(dirty_suffix(Some(false)), "");
        assert_eq!(dirty_suffix(None), " (working tree unknown)");
    }

    #[test]
    fn json_serialization_failure_maps_to_a_safe_internal_error() {
        struct FailsSerialization;
        impl Serialize for FailsSerialization {
            fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
                Err(serde::ser::Error::custom("private serializer detail"))
            }
        }

        let error = pretty_json(&FailsSerialization).unwrap_err();
        assert!(
            error
                .message()
                .contains("cannot serialize build information")
        );
        assert!(!error.message().contains("private serializer detail"));
    }
}
