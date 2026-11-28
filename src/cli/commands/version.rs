//! Build identity presentation for the version command.

/// Renders the build identity block, or pretty JSON with `--json`.
pub(in crate::cli) fn render_build_info(json: bool) -> String {
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
        return serde_json::to_string_pretty(&info)
            .unwrap_or_else(|error| format!("{{\"ok\":false,\"error\":\"{error}\"}}"));
    }
    let revision = match info.dirty {
        Some(true) => format!("{} (dirty)", info.revision),
        Some(false) => info.revision.to_owned(),
        None => format!("{} (working tree unknown)", info.revision),
    };
    [
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
    .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_output_lists_identity_fields() {
        let rendered = render_build_info(false);
        assert!(rendered.starts_with("AsterYggdrasil "));
        assert!(rendered.contains("Revision:  "));
        assert!(rendered.contains("Target:    "));
        assert!(rendered.contains("Profile:   "));
        assert!(rendered.contains("Features:  "));
        assert!(rendered.contains("Built:     "));
    }

    #[test]
    fn json_output_serializes_identity_fields() {
        let rendered = render_build_info(true);
        let parsed: serde_json::Value =
            serde_json::from_str(&rendered).expect("json output must parse");
        assert_eq!(parsed["product"], "AsterYggdrasil");
        assert_eq!(parsed["version"], env!("CARGO_PKG_VERSION"));
        assert!(parsed["revision"].is_string());
        assert!(parsed["features"].is_array());
    }
}
