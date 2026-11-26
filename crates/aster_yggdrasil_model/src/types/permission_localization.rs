//! Backend-owned translations for permission labels.
//!
//! Permission descriptors carry stable message ids (`label_i18n_key`); this
//! module ships the matching localized text as compile-time data and serves
//! it through an authenticated endpoint. The frontend installs the resolved
//! message map into its i18n instance, so adding a permission never requires
//! a frontend translation change and a missing translation fails at startup
//! instead of degrading silently for users.
//!
//! The declaration format intentionally mirrors the storage-connector
//! localization contract so a future plugin registry can register external
//! descriptors and their translations through the same validated path.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

const MAX_LOCALES: usize = 32;
const MAX_MESSAGES: usize = 512;
const MAX_MESSAGE_ID_LENGTH: usize = 128;
const MAX_MESSAGE_LENGTH: usize = 4_096;
const MAX_REVISION_LENGTH: usize = 128;

/// Domain tag mixed into the revision hash so permission revisions never
/// collide with revisions derived for other localization domains.
const REVISION_DOMAIN: &str = "aster.yggdrasil.permissions";

/// Serving namespace advertised to clients installing the bundle.
pub const PERMISSION_LOCALIZATION_NAMESPACE: &str = "permissions_backend";

/// Default locale used when no supported locale matches the request.
pub const PERMISSION_DEFAULT_LOCALE: &str = "en";

/// One permission-owned message and all translations shipped for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PermissionLocalizationMessage<'a> {
    /// Stable descriptor message id; validated against the id charset and
    /// length limits by [`PermissionLocalization::from_messages`].
    pub message_id: &'a str,
    /// All locale variants shipped for this message; each locale may appear
    /// at most once.
    pub translations: &'a [PermissionLocalizationTranslation<'a>],
}

impl<'a> PermissionLocalizationMessage<'a> {
    /// Builds a message declaration from static slices.
    #[must_use]
    pub const fn new(
        message_id: &'a str,
        translations: &'a [PermissionLocalizationTranslation<'a>],
    ) -> Self {
        Self {
            message_id,
            translations,
        }
    }
}

/// One locale variant in a permission message declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PermissionLocalizationTranslation<'a> {
    /// BCP-47-style locale tag.
    pub locale: &'a str,
    /// Localized text; may contain `{{placeholder}}` interpolations that
    /// must match the default locale's placeholders.
    pub value: &'a str,
}

impl<'a> PermissionLocalizationTranslation<'a> {
    /// Builds one locale variant from static slices.
    #[must_use]
    pub const fn new(locale: &'a str, value: &'a str) -> Self {
        Self { locale, value }
    }
}

/// Declare permission localization in Rust without manually assembling maps.
/// The locale-list form stays open so adding a product locale never changes
/// the declaration contract.
#[macro_export]
macro_rules! permission_message {
    ($message_id:literal, $en:literal, $zh:literal $(,)?) => {
        $crate::types::permission_localization::PermissionLocalizationMessage::new(
            $message_id,
            &[
                $crate::types::permission_localization::PermissionLocalizationTranslation::new("en", $en),
                $crate::types::permission_localization::PermissionLocalizationTranslation::new("zh", $zh),
            ],
        )
    };
    ($message_id:literal; $($locale:literal => $value:literal),+ $(,)?) => {
        $crate::types::permission_localization::PermissionLocalizationMessage::new(
            $message_id,
            &[
                $($crate::types::permission_localization::PermissionLocalizationTranslation::new($locale, $value)),+
            ],
        )
    };
}

/// Validated localization resource for permission labels.
///
/// Construction always runs validation, so an instance guarantees: non-empty
/// revision, 1..=`MAX_LOCALES` locales with the default locale present, every
/// locale carrying the same message id set, matching interpolation
/// placeholders across locales, and per-message id/text limits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionLocalization {
    default_locale: LocaleTag,
    revision: String,
    resources: BTreeMap<LocaleTag, BTreeMap<String, String>>,
}

impl PermissionLocalization {
    /// Builds a validated resource from the static declarations.
    ///
    /// The revision is derived from normalized locale/message ordering, so it
    /// stays deterministic regardless of declaration order.
    pub fn from_messages<'a>(
        default_locale: &str,
        messages: impl IntoIterator<Item = &'a PermissionLocalizationMessage<'a>>,
    ) -> Result<Self, PermissionLocalizationError> {
        let default_locale = LocaleTag::parse(default_locale)?;
        let mut message_ids = BTreeSet::new();
        let mut resources = BTreeMap::<LocaleTag, BTreeMap<String, String>>::new();

        for message in messages {
            if !message_ids.insert(message.message_id) {
                return Err(PermissionLocalizationError(format!(
                    "permission localization declares message '{}' more than once",
                    message.message_id
                )));
            }
            let mut translated_locales = BTreeSet::new();
            for translation in message.translations {
                let locale = LocaleTag::parse(translation.locale)?;
                if !translated_locales.insert(locale.clone()) {
                    return Err(PermissionLocalizationError(format!(
                        "permission message '{}' declares locale '{}' more than once",
                        message.message_id,
                        locale.as_str()
                    )));
                }
                resources.entry(locale).or_default().insert(
                    message.message_id.to_string(),
                    translation.value.to_string(),
                );
            }
        }

        let mut revision_hasher = Sha256::new();
        revision_hasher.update(REVISION_DOMAIN.as_bytes());
        revision_hasher.update([0]);
        revision_hasher.update(default_locale.as_str().as_bytes());
        revision_hasher.update([0]);
        for (locale, localized_messages) in &resources {
            revision_hasher.update(locale.as_str().as_bytes());
            revision_hasher.update([0]);
            for (message_id, value) in localized_messages {
                revision_hasher.update(message_id.as_bytes());
                revision_hasher.update([0]);
                revision_hasher.update(value.as_bytes());
                revision_hasher.update([0]);
            }
        }

        Self::new(
            default_locale,
            hex::encode(revision_hasher.finalize()),
            resources,
        )
    }

    /// Builds a resource from already-assembled maps and validates it.
    ///
    /// # Errors
    ///
    /// Returns [`PermissionLocalizationError`] when the revision, locale
    /// coverage, key parity, or per-message limits fail validation.
    pub fn new(
        default_locale: LocaleTag,
        revision: impl Into<String>,
        resources: BTreeMap<LocaleTag, BTreeMap<String, String>>,
    ) -> Result<Self, PermissionLocalizationError> {
        let localization = Self {
            default_locale,
            revision: revision.into(),
            resources,
        };
        localization.validate()?;
        Ok(localization)
    }

    /// Returns the resource revision; equal revisions imply identical
    /// localized content.
    #[must_use]
    pub fn revision(&self) -> &str {
        &self.revision
    }

    /// Returns the default locale of this resource.
    #[must_use]
    pub fn default_locale(&self) -> &LocaleTag {
        &self.default_locale
    }

    /// Returns the locale actually served for the requested tag.
    #[must_use]
    pub fn resolved_locale(&self, requested_locale: &LocaleTag) -> LocaleTag {
        self.resolve_locale(requested_locale)
    }

    /// Resolves the best matching locale and returns its full message map.
    ///
    /// Resolution walks the requested tag up through its parent tags
    /// (`zh-CN` -> `zh`) with ASCII case-insensitive comparison and falls
    /// back to the default locale when nothing matches, so the returned
    /// bundle is always complete.
    #[must_use]
    pub fn bundle(&self, requested_locale: &LocaleTag) -> PermissionLocalizationBundle {
        let resolved_locale = self.resolve_locale(requested_locale);
        PermissionLocalizationBundle {
            namespace: PERMISSION_LOCALIZATION_NAMESPACE.to_string(),
            requested_locale: requested_locale.clone(),
            resolved_locale: resolved_locale.clone(),
            revision: self.revision.clone(),
            messages: self.resources[&resolved_locale].clone(),
        }
    }

    /// Checks that every referenced message id exists in the default locale.
    ///
    /// The registry calls this during construction with the label keys its
    /// descriptors reference; a missing id fails the build.
    ///
    /// # Errors
    ///
    /// Returns [`PermissionLocalizationError`] naming the missing id.
    pub fn validate_message_ids<'a>(
        &self,
        message_ids: impl IntoIterator<Item = &'a str>,
    ) -> Result<(), PermissionLocalizationError> {
        let default_messages = &self.resources[&self.default_locale];
        for message_id in message_ids {
            if !default_messages.contains_key(message_id) {
                return Err(PermissionLocalizationError(format!(
                    "permission localization is missing descriptor message id '{message_id}'"
                )));
            }
        }
        Ok(())
    }

    fn resolve_locale(&self, requested_locale: &LocaleTag) -> LocaleTag {
        let mut candidate = requested_locale.as_str();
        loop {
            if let Some(locale) = self
                .resources
                .keys()
                .find(|locale| locale.as_str().eq_ignore_ascii_case(candidate))
            {
                return locale.clone();
            }
            let Some((parent, _)) = candidate.rsplit_once('-') else {
                break;
            };
            candidate = parent;
        }
        self.default_locale.clone()
    }

    fn validate(&self) -> Result<(), PermissionLocalizationError> {
        if self.revision.trim().is_empty() || self.revision.len() > MAX_REVISION_LENGTH {
            return Err(PermissionLocalizationError(format!(
                "permission localization revision must be 1..={MAX_REVISION_LENGTH} bytes"
            )));
        }
        if self.resources.is_empty() || self.resources.len() > MAX_LOCALES {
            return Err(PermissionLocalizationError(format!(
                "permission localization must contain 1..={MAX_LOCALES} locales"
            )));
        }
        let Some(default_messages) = self.resources.get(&self.default_locale) else {
            return Err(PermissionLocalizationError(format!(
                "permission localization is missing default locale '{}'",
                self.default_locale.as_str()
            )));
        };
        validate_messages(&self.default_locale, default_messages)?;
        let default_keys = default_messages.keys().collect::<BTreeSet<_>>();

        let default_placeholders = default_messages
            .iter()
            .map(|(key, value)| (key, interpolation_placeholders(value)))
            .collect::<BTreeMap<_, _>>();

        for (locale, messages) in &self.resources {
            validate_messages(locale, messages)?;
            let keys = messages.keys().collect::<BTreeSet<_>>();
            if keys != default_keys {
                return Err(PermissionLocalizationError(format!(
                    "permission locale '{}' message ids differ from default locale '{}'",
                    locale.as_str(),
                    self.default_locale.as_str()
                )));
            }
            for (key, value) in messages {
                if interpolation_placeholders(value) != default_placeholders[key] {
                    return Err(PermissionLocalizationError(format!(
                        "permission locale '{}' message '{}' uses different interpolation placeholders",
                        locale.as_str(),
                        key
                    )));
                }
            }
        }
        Ok(())
    }
}

/// One permission registry's resolved message set for a locale request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct PermissionLocalizationBundle {
    /// Serving namespace; clients install the messages under it.
    pub namespace: String,
    /// Locale the client asked for, before fallback.
    pub requested_locale: LocaleTag,
    /// Locale actually served after fallback resolution.
    pub resolved_locale: LocaleTag,
    /// Resource revision the messages belong to.
    pub revision: String,
    /// Message id to localized text for the resolved locale.
    pub messages: BTreeMap<String, String>,
}

/// Validation failure for a permission localization resource.
///
/// The message text names the offending id, locale, or limit so a failed
/// startup points straight at the broken declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionLocalizationError(String);

impl std::fmt::Display for PermissionLocalizationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for PermissionLocalizationError {}

/// A BCP-47-style locale tag (`en`, `zh-CN`), validated at the segment level.
///
/// Comparison is ASCII case-insensitive (BCP-47 tags are case-insensitive),
/// while the canonical original spelling is preserved for display.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[serde(try_from = "String")]
pub struct LocaleTag(String);

impl PartialEq for LocaleTag {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

impl Eq for LocaleTag {}

impl PartialOrd for LocaleTag {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LocaleTag {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0
            .to_ascii_lowercase()
            .cmp(&other.0.to_ascii_lowercase())
    }
}

impl std::hash::Hash for LocaleTag {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.to_ascii_lowercase().hash(state);
    }
}

impl LocaleTag {
    /// Parses and validates a locale tag.
    ///
    /// # Errors
    ///
    /// Returns [`PermissionLocalizationError`] when the tag is empty,
    /// oversized, or carries malformed segments.
    pub fn parse(raw: &str) -> Result<Self, PermissionLocalizationError> {
        if raw.is_empty() || raw.len() > 35 {
            return Err(PermissionLocalizationError(format!(
                "invalid locale tag '{raw}'"
            )));
        }
        let valid_segment = |segment: &str| {
            !segment.is_empty()
                && segment.len() <= 8
                && segment.bytes().all(|byte| byte.is_ascii_alphanumeric())
        };
        if !raw.split('-').all(valid_segment) {
            return Err(PermissionLocalizationError(format!(
                "invalid locale tag '{raw}'"
            )));
        }
        Ok(Self(raw.to_string()))
    }

    /// Canonical string form of this tag.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for LocaleTag {
    type Error = PermissionLocalizationError;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        Self::parse(&raw)
    }
}

impl std::fmt::Display for LocaleTag {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

fn validate_messages(
    locale: &LocaleTag,
    messages: &BTreeMap<String, String>,
) -> Result<(), PermissionLocalizationError> {
    if messages.is_empty() || messages.len() > MAX_MESSAGES {
        return Err(PermissionLocalizationError(format!(
            "permission locale '{}' must contain 1..={MAX_MESSAGES} messages",
            locale.as_str()
        )));
    }
    for (message_id, value) in messages {
        // Keys carrying a segment the frontend installer filters out must be
        // rejected here: otherwise the registry builds successfully and the
        // label silently degrades to English for non-English users.
        if message_id
            .split('.')
            .any(|segment| matches!(segment, "__proto__" | "constructor" | "prototype"))
        {
            return Err(PermissionLocalizationError(format!(
                "permission locale '{}' has message id '{}' with a forbidden segment",
                locale.as_str(),
                message_id
            )));
        }
        if message_id.is_empty()
            || message_id.len() > MAX_MESSAGE_ID_LENGTH
            // Alphanumerics plus separators: the frontend bundle keys use
            // camelCase segments (e.g. `admin.users.operatorScope.audit`).
            || !message_id.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || b"._-".contains(&byte)
            })
        {
            return Err(PermissionLocalizationError(format!(
                "permission locale '{}' has invalid message id '{}'",
                locale.as_str(),
                message_id
            )));
        }
        if value.trim().is_empty()
            || value.len() > MAX_MESSAGE_LENGTH
            || value
                .chars()
                .any(|character| character.is_control() && character != '\n' && character != '\t')
        {
            return Err(PermissionLocalizationError(format!(
                "permission locale '{}' message '{}' has invalid text",
                locale.as_str(),
                message_id
            )));
        }
    }
    Ok(())
}

fn interpolation_placeholders(value: &str) -> BTreeSet<&str> {
    let mut placeholders = BTreeSet::new();
    let mut remaining = value;
    while let Some(start) = remaining.find("{{") {
        remaining = &remaining[start + 2..];
        let Some(end) = remaining.find("}}") else {
            break;
        };
        let placeholder = remaining[..end].trim();
        if !placeholder.is_empty() {
            placeholders.insert(placeholder);
        }
        remaining = &remaining[end + 2..];
    }
    placeholders
}

/// Translations for every built-in permission label, keyed by the same
/// message ids the descriptors reference.
pub const BUILTIN_PERMISSION_MESSAGES: &[PermissionLocalizationMessage<'static>] = &[
    crate::permission_message!(
        "admin.users.operatorScope.overview",
        "Admin overview",
        "后台概览"
    ),
    crate::permission_message!("admin.users.operatorScope.users", "Users", "用户管理"),
    crate::permission_message!("admin.users.operatorScope.profiles", "Profiles", "角色档案"),
    crate::permission_message!(
        "admin.users.operatorScope.texture_library",
        "Texture library",
        "材质库"
    ),
    crate::permission_message!("admin.users.operatorScope.audit", "Audit logs", "审计日志"),
    crate::permission_message!(
        "admin.users.operatorScope.tasks",
        "Background tasks",
        "后台任务"
    ),
    crate::permission_message!(
        "admin.users.operatorScope.settings",
        "System settings",
        "系统设置"
    ),
    crate::permission_message!(
        "admin.users.operatorScope.external_auth",
        "External auth",
        "外部登录"
    ),
    crate::permission_message!(
        "admin.users.bans.scope.yggdrasil_access",
        "Yggdrasil login access",
        "Yggdrasil 登录访问"
    ),
    crate::permission_message!(
        "admin.users.bans.scope.yggdrasil_join",
        "Yggdrasil server join",
        "Yggdrasil 加入服务器"
    ),
    crate::permission_message!(
        "admin.users.bans.scope.minecraft_profile_manage",
        "Profile management",
        "角色档案管理"
    ),
    crate::permission_message!(
        "admin.users.bans.scope.texture_upload",
        "Texture upload and wardrobe",
        "材质上传与衣柜管理"
    ),
    crate::permission_message!(
        "admin.users.bans.scope.texture_library_interact",
        "Texture library interaction",
        "材质库交互"
    ),
];

/// Builds the built-in localization resource backing the registry.
///
/// # Errors
///
/// Returns [`PermissionLocalizationError`] when the built-in table violates
/// localization invariants; the unit tests keep the table valid.
pub fn builtin_permission_localization()
-> Result<PermissionLocalization, PermissionLocalizationError> {
    PermissionLocalization::from_messages(PERMISSION_DEFAULT_LOCALE, BUILTIN_PERMISSION_MESSAGES)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_table_is_valid_and_bilingual() {
        let localization = builtin_permission_localization().expect("built-in table must be valid");
        assert_eq!(localization.revision().len(), 64);
        assert_eq!(localization.default_locale().as_str(), "en");

        let zh_bundle = localization.bundle(&LocaleTag::parse("zh-CN").expect("valid"));
        assert_eq!(zh_bundle.resolved_locale.as_str(), "zh");
        assert_eq!(
            zh_bundle.messages.get("admin.users.operatorScope.overview"),
            Some(&"后台概览".to_string())
        );
        assert_eq!(zh_bundle.messages.len(), BUILTIN_PERMISSION_MESSAGES.len());

        let en_bundle = localization.bundle(&LocaleTag::parse("fr-FR").expect("valid"));
        assert_eq!(en_bundle.resolved_locale.as_str(), "en");
        assert_eq!(
            en_bundle
                .messages
                .get("admin.users.bans.scope.texture_upload"),
            Some(&"Texture upload and wardrobe".to_string())
        );
    }

    #[test]
    fn descriptor_keys_all_resolve_in_the_table() {
        let localization = builtin_permission_localization().expect("built-in table must be valid");
        let referenced: Vec<&str> = crate::types::permission::BUILTIN_DESCRIPTORS
            .iter()
            .map(|descriptor| descriptor.label_i18n_key)
            .collect();
        localization
            .validate_message_ids(referenced.iter().copied())
            .expect("every descriptor label key must resolve");
    }

    #[test]
    fn rejects_key_drift_placeholder_mismatch_and_duplicates() {
        let key_drift = PermissionLocalization::from_messages(
            "en",
            &[
                crate::permission_message!("a", "A", "甲"),
                crate::permission_message!("b"; "en" => "B"),
            ],
        )
        .unwrap_err();
        assert!(key_drift.to_string().contains("message ids differ"));

        let placeholders = PermissionLocalization::from_messages(
            "en",
            &[
                crate::permission_message!("a", "Hello {{name}}", "你好 {{name}}"),
                crate::permission_message!("a", "Duplicate", "重复"),
            ],
        )
        .unwrap_err();
        assert!(placeholders.to_string().contains("more than once"));

        let missing_default = PermissionLocalization::new(
            LocaleTag::parse("en").expect("valid"),
            "1",
            BTreeMap::from([(
                LocaleTag::parse("zh").expect("valid"),
                BTreeMap::from([("a".to_string(), "甲".to_string())]),
            )]),
        )
        .unwrap_err();
        assert!(missing_default.to_string().contains("default locale"));

        let bad_tag = LocaleTag::parse("zh__CN").unwrap_err();
        assert!(bad_tag.to_string().contains("invalid locale tag"));
    }

    #[test]
    fn placeholder_mismatch_fails_construction() {
        let error = PermissionLocalization::from_messages(
            "en",
            &[
                crate::permission_message!("a", "Hello {{name}}", "你好 {{name}}"),
                crate::permission_message!(
                    "b";
                    "en" => "Open {{path}}",
                    "zh" => "打开 {{target}}",
                ),
            ],
        )
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("different interpolation placeholders")
        );
    }

    #[test]
    fn duplicate_locale_in_one_message_fails() {
        let error = PermissionLocalization::from_messages(
            "en",
            &[crate::permission_message!(
                "a";
                "en" => "A",
                "EN" => "A again",
            )],
        )
        .unwrap_err();
        assert!(error.to_string().contains("more than once"));
    }

    #[test]
    fn invalid_default_locale_fails_parse() {
        let error = PermissionLocalization::from_messages(
            "zh__CN",
            &[crate::permission_message!("a", "A", "甲")],
        )
        .unwrap_err();
        assert!(error.to_string().contains("invalid locale tag"));
    }

    #[test]
    fn structural_validation_rejects_bad_revisions_and_bounds() {
        let make = |id: &str, value: &str| {
            BTreeMap::from([(
                LocaleTag::parse("en").expect("valid"),
                BTreeMap::from([(id.to_string(), value.to_string())]),
            )])
        };

        let blank_revision = PermissionLocalization::new(
            LocaleTag::parse("en").expect("valid"),
            "  ",
            make("a", "A"),
        )
        .unwrap_err();
        assert!(blank_revision.to_string().contains("revision"));

        let empty_resources = PermissionLocalization::new(
            LocaleTag::parse("en").expect("valid"),
            "1",
            BTreeMap::new(),
        )
        .unwrap_err();
        assert!(empty_resources.to_string().contains("1..=32 locales"));

        let missing_default = PermissionLocalization::new(
            LocaleTag::parse("en").expect("valid"),
            "1",
            BTreeMap::from([(
                LocaleTag::parse("zh").expect("valid"),
                BTreeMap::from([("a".to_string(), "甲".to_string())]),
            )]),
        )
        .unwrap_err();
        assert!(missing_default.to_string().contains("default locale"));
    }

    #[test]
    fn message_content_limits_are_enforced() {
        let make = |id: &str, value: &str| {
            BTreeMap::from([(
                LocaleTag::parse("en").expect("valid"),
                BTreeMap::from([(id.to_string(), value.to_string())]),
            )])
        };

        let empty_id =
            PermissionLocalization::new(LocaleTag::parse("en").expect("valid"), "1", make("", "A"))
                .unwrap_err();
        assert!(empty_id.to_string().contains("invalid message id"));

        let bad_chars = PermissionLocalization::new(
            LocaleTag::parse("en").expect("valid"),
            "1",
            make("Bad Key", "A"),
        )
        .unwrap_err();
        assert!(bad_chars.to_string().contains("invalid message id"));

        let long_id = PermissionLocalization::new(
            LocaleTag::parse("en").expect("valid"),
            "1",
            make(&"a".repeat(129), "A"),
        )
        .unwrap_err();
        assert!(long_id.to_string().contains("invalid message id"));

        let blank_value = PermissionLocalization::new(
            LocaleTag::parse("en").expect("valid"),
            "1",
            make("a", "   "),
        )
        .unwrap_err();
        assert!(blank_value.to_string().contains("invalid text"));

        let control_chars = PermissionLocalization::new(
            LocaleTag::parse("en").expect("valid"),
            "1",
            make("a", "bad\u{7}text"),
        )
        .unwrap_err();
        assert!(control_chars.to_string().contains("invalid text"));

        let long_value = PermissionLocalization::new(
            LocaleTag::parse("en").expect("valid"),
            "1",
            make("a", &"x".repeat(4_097)),
        )
        .unwrap_err();
        assert!(long_value.to_string().contains("invalid text"));
    }

    #[test]
    fn prefix_pairs_are_legal_and_forbidden_segments_are_rejected() {
        // The frontend installs messages flat (empty key separator), so a
        // prefix pair is two independent keys, not a node conflict.
        let localization = PermissionLocalization::from_messages(
            "en",
            &[
                crate::permission_message!("plugin.render", "Render", "渲染"),
                crate::permission_message!("plugin.render.export", "Export", "导出"),
            ],
        )
        .expect("prefix pairs are legal in the flat store");
        let bundle = localization.bundle(&LocaleTag::parse("en").expect("valid"));
        assert_eq!(
            bundle.messages.get("plugin.render"),
            Some(&"Render".to_string())
        );
        assert_eq!(
            bundle.messages.get("plugin.render.export"),
            Some(&"Export".to_string())
        );

        // Keys the frontend installer filters out must fail the build,
        // otherwise the label silently degrades to English.
        let error = PermissionLocalization::from_messages(
            "en",
            &[
                crate::permission_message!("plugin.render", "Render", "渲染"),
                crate::permission_message!("plugin.constructor.export", "Export", "导出"),
            ],
        )
        .unwrap_err();
        assert!(error.to_string().contains("forbidden segment"));
    }

    #[test]
    fn validate_message_ids_names_the_missing_key() {
        let localization = builtin_permission_localization().expect("built-in table");
        let error = localization
            .validate_message_ids(["admin.users.operatorScope.does_not_exist"])
            .unwrap_err();
        assert!(error.to_string().contains("does_not_exist"));
    }

    #[test]
    fn locale_tag_traits_and_limits_are_covered() {
        // Case-insensitive equality and ordering.
        let en = LocaleTag::parse("en").expect("valid");
        let en_upper = LocaleTag::parse("EN").expect("valid");
        assert_eq!(en, en_upper);
        assert!(en == en_upper);

        // Display and the serde TryFrom path.
        assert_eq!(en.to_string(), "en");
        let from_serde: LocaleTag = serde_json::from_str("\"zh-CN\"").expect("valid");
        assert_eq!(from_serde.as_str(), "zh-CN");

        // Overlong and malformed tags fail at the segment level.
        assert!(LocaleTag::parse(&"a".repeat(36)).is_err());
        assert!(LocaleTag::parse("zh__CN").is_err());
        assert!(LocaleTag::parse("").is_err());

        // Hash-based collections see case variants as one entry.
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(LocaleTag::parse("EN").expect("valid"));
        assert!(set.contains(&LocaleTag::parse("en").expect("valid")));

        // The public resolved_locale helper mirrors bundle()'s resolution.
        let localization = builtin_permission_localization().expect("built-in");
        let resolved = localization.resolved_locale(&LocaleTag::parse("zh-SG").expect("valid"));
        assert_eq!(resolved.as_str(), "zh");
    }

    #[test]
    fn empty_locale_map_in_direct_construction_is_rejected() {
        let error = PermissionLocalization::new(
            LocaleTag::parse("en").expect("valid"),
            "1",
            BTreeMap::from([
                (
                    LocaleTag::parse("en").expect("valid"),
                    BTreeMap::from([("a".to_string(), "A".to_string())]),
                ),
                (LocaleTag::parse("zh").expect("valid"), BTreeMap::new()),
            ]),
        )
        .unwrap_err();
        assert!(error.to_string().contains("1..=512 messages"));
    }

    #[test]
    fn resolve_walks_single_segment_and_deep_parent_chains() {
        let localization = builtin_permission_localization().expect("built-in");
        // A single unsupported segment walks straight to the default.
        let bundle = localization.bundle(&LocaleTag::parse("xx").expect("valid"));
        assert_eq!(bundle.resolved_locale.as_str(), "en");
        // A deep chain strips subtags until a supported locale matches.
        let bundle = localization.bundle(&LocaleTag::parse("zh-Hant-TW").expect("valid"));
        assert_eq!(bundle.resolved_locale.as_str(), "zh");
    }

    #[test]
    fn revision_is_order_independent() {
        let forward = PermissionLocalization::from_messages(
            "en",
            &[
                crate::permission_message!("a", "A", "甲"),
                crate::permission_message!("b", "B", "乙"),
            ],
        )
        .expect("valid");
        let reverse = PermissionLocalization::from_messages(
            "en",
            &[
                crate::permission_message!("b", "B", "乙"),
                crate::permission_message!("a", "A", "甲"),
            ],
        )
        .expect("valid");
        assert_eq!(forward.revision(), reverse.revision());
    }
}
