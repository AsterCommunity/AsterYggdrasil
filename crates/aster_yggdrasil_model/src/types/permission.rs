//! Open permission identifiers for admin-area grants and player capability
//! bans.
//!
//! Both authorization systems share one identifier syntax: a reversed-DNS
//! namespace split by a semantic boundary segment — `*.permission.*` for
//! staff-facing admin-panel areas (grant semantics, no expiry) and
//! `*.capability.*` for player abilities (ban deny targets, may expire).
//! The namespace boundary is the semantic boundary, so the two systems can
//! never be confused at the identifier level.
//!
//! Built-in permissions live in the [`BUILTIN_NAMESPACE`] namespace and are
//! exposed as `const` items under [`permission`] and [`capability`]. Third
//! parties will later own their own namespaces; until the plugin registry
//! ships, only built-in identifiers are accepted by [`PermissionRegistry`].

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use super::permission_localization::{
    BUILTIN_PERMISSION_MESSAGES, PermissionLocalization, PermissionLocalizationError,
    PermissionLocalizationMessage,
};

use sea_orm::DeriveValueType;
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

/// Reverse-DNS namespace reserved for permissions defined in this codebase.
pub const BUILTIN_NAMESPACE: &str = "com.astercosm.yggdrasil";

/// Maximum byte length of a permission identifier.
pub const PERMISSION_ID_MAX_LEN: usize = 256;

/// Semantic boundary segment separating the owner namespace from the
/// admin-area suffix.
const PERMISSION_BOUNDARY: &str = ".permission.";

/// Semantic boundary segment separating the owner namespace from the player
/// capability suffix.
const CAPABILITY_BOUNDARY: &str = ".capability.";

/// Validated permission identifier shared by admin-area grants and player
/// capability bans.
///
/// The syntax is `{namespace}.permission.{segments}` or
/// `{namespace}.capability.{segments}` where `namespace` is at least two
/// reversed-DNS segments and every segment matches `[a-z][a-z0-9_]*`. The
/// total length is capped at [`PERMISSION_ID_MAX_LEN`] bytes.
///
/// Identifiers coming from compile-time built-ins use
/// [`PermissionId::builtin`]; identifiers parsed from external input go
/// through [`PermissionId::new`], which validates the syntax. Semantic
/// registration checks live in [`PermissionRegistry`].
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, DeriveValueType,
)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(
    value_type = "String",
    column_type = "String(sea_orm::sea_query::StringLen::N(256))"
)]
#[serde(try_from = "String")]
pub struct PermissionId(Cow<'static, str>);

/// Failure modes when parsing a [`PermissionId`] from raw input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionIdError {
    /// The identifier was empty.
    Empty,
    /// The identifier exceeded [`PERMISSION_ID_MAX_LEN`] bytes.
    TooLong {
        /// Actual byte length.
        len: usize,
    },
    /// The identifier carried no `.permission.` or `.capability.` boundary.
    MissingNamespace,
    /// The identifier carried both semantic boundaries, so its kind is
    /// ambiguous.
    AmbiguousNamespace,
    /// A segment of the identifier did not match `[a-z][a-z0-9_]*`.
    InvalidSegment {
        /// The offending segment.
        segment: String,
    },
}

impl fmt::Display for PermissionIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "permission id must not be empty"),
            Self::TooLong { len } => write!(
                f,
                "permission id must not exceed {PERMISSION_ID_MAX_LEN} bytes (got {len})"
            ),
            Self::MissingNamespace => write!(
                f,
                "permission id must contain a '{PERMISSION_BOUNDARY}' or \
                 '{CAPABILITY_BOUNDARY}' namespace boundary"
            ),
            Self::AmbiguousNamespace => write!(
                f,
                "permission id must not contain both the \
                 '{PERMISSION_BOUNDARY}' and '{CAPABILITY_BOUNDARY}' boundaries"
            ),
            Self::InvalidSegment { segment } => {
                write!(f, "permission id segment '{segment}' is invalid")
            }
        }
    }
}

impl std::error::Error for PermissionIdError {}

impl PermissionId {
    /// Wraps a compile-time known built-in identifier without re-validating.
    ///
    /// Built-in constants are proven valid by the unit tests of
    /// [`BUILTIN_DESCRIPTORS`]; this constructor is `const` so built-in
    /// permissions can be declared as `const` items.
    #[must_use]
    pub const fn builtin(id: &'static str) -> Self {
        Self(Cow::Borrowed(id))
    }

    /// Parses a permission identifier from raw input, validating the syntax.
    ///
    /// # Errors
    ///
    /// Returns [`PermissionIdError`] when the identifier is empty, too long,
    /// missing its namespace boundary, or carries an invalid segment.
    pub fn new(raw: impl Into<String>) -> Result<Self, PermissionIdError> {
        let raw = raw.into();
        validate_permission_id(&raw)?;
        Ok(Self(Cow::Owned(raw)))
    }

    /// Canonical string form of this identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Semantic kind encoded by the namespace boundary of this identifier.
    ///
    /// The syntax guarantees exactly one boundary, so this never fails.
    #[must_use]
    pub fn kind(&self) -> PermissionKind {
        if self.0.contains(PERMISSION_BOUNDARY) {
            PermissionKind::AdminArea
        } else {
            PermissionKind::Capability
        }
    }

    /// Whether this identifier lives in the built-in
    /// [`BUILTIN_NAMESPACE`] namespace.
    ///
    /// Matches on the segment boundary, so look-alike namespaces such as
    /// `com.astercosm.yggdrasil2` stay external.
    #[must_use]
    pub fn is_builtin(&self) -> bool {
        match self.0.strip_prefix(BUILTIN_NAMESPACE) {
            Some(rest) => rest.starts_with('.'),
            None => false,
        }
    }
}

impl TryFrom<String> for PermissionId {
    type Error = PermissionIdError;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        Self::new(raw)
    }
}

impl FromStr for PermissionId {
    type Err = PermissionIdError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        Self::new(raw)
    }
}

impl fmt::Display for PermissionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Validates the shared permission identifier syntax.
///
/// Returns the semantic kind selected by the namespace boundary.
fn validate_permission_id(raw: &str) -> Result<PermissionKind, PermissionIdError> {
    if raw.is_empty() {
        return Err(PermissionIdError::Empty);
    }
    if raw.len() > PERMISSION_ID_MAX_LEN {
        return Err(PermissionIdError::TooLong { len: raw.len() });
    }

    let permission_pos = raw.find(PERMISSION_BOUNDARY);
    let capability_pos = raw.find(CAPABILITY_BOUNDARY);
    let (boundary, kind) = match (permission_pos, capability_pos) {
        (Some(_), Some(_)) => return Err(PermissionIdError::AmbiguousNamespace),
        (Some(_), None) => (PERMISSION_BOUNDARY, PermissionKind::AdminArea),
        (None, Some(_)) => (CAPABILITY_BOUNDARY, PermissionKind::Capability),
        (None, None) => return Err(PermissionIdError::MissingNamespace),
    };

    let namespace = &raw[..permission_pos.or(capability_pos).unwrap_or(0)];
    let suffix = &raw[namespace.len() + boundary.len()..];
    validate_namespace(namespace)?;
    validate_suffix(suffix)?;
    Ok(kind)
}

/// Validates the reversed-DNS namespace before the semantic boundary.
fn validate_namespace(namespace: &str) -> Result<(), PermissionIdError> {
    let mut segments = namespace.split('.');
    let mut count = 0;
    for segment in segments.by_ref() {
        if !is_valid_segment(segment) {
            return Err(PermissionIdError::InvalidSegment {
                segment: segment.to_string(),
            });
        }
        count += 1;
    }
    if count < 2 {
        // A reversed-DNS namespace needs at least two segments; single
        // segments would collapse the boundary into the first suffix segment.
        return Err(PermissionIdError::InvalidSegment {
            segment: namespace.to_string(),
        });
    }
    Ok(())
}

/// Validates the suffix segments after the semantic boundary.
fn validate_suffix(suffix: &str) -> Result<(), PermissionIdError> {
    for segment in suffix.split('.') {
        if !is_valid_segment(segment) {
            return Err(PermissionIdError::InvalidSegment {
                segment: segment.to_string(),
            });
        }
    }
    Ok(())
}

/// Whether one identifier segment matches `[a-z][a-z0-9_]*`.
fn is_valid_segment(segment: &str) -> bool {
    let mut chars = segment.chars();
    match chars.next() {
        Some(first) if first.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Semantic class of a permission, encoded by its namespace boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum PermissionKind {
    /// Staff-facing admin-panel area, granted to operator accounts.
    AdminArea,
    /// Player ability targeted by capability bans.
    Capability,
}

impl PermissionKind {
    /// Canonical storage/serialization string for this kind.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AdminArea => "admin_area",
            Self::Capability => "capability",
        }
    }
}

impl fmt::Display for PermissionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Metadata describing one registered permission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct PermissionDescriptor {
    /// Validated permission identifier.
    pub id: PermissionId,
    /// Semantic class; must agree with the namespace boundary of `id`.
    pub kind: PermissionKind,
    /// Frontend i18n message key rendering the localized label.
    pub label_i18n_key: &'static str,
    /// English fallback label used when the i18n bundle lacks the key.
    pub label: &'static str,
    /// UI grouping hint (for example `admin_panel` or the capability domain).
    pub group: &'static str,
}

/// Failure modes when constructing a [`PermissionRegistry`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionRegistryError {
    /// Two descriptors carried the same identifier.
    Duplicate {
        /// The duplicated identifier.
        id: PermissionId,
    },
    /// A descriptor's declared kind disagreed with the namespace boundary of
    /// its identifier.
    KindMismatch {
        /// The offending identifier.
        id: PermissionId,
        /// The kind declared on the descriptor.
        declared: PermissionKind,
    },
    /// An extra descriptor occupied the built-in namespace, which only
    /// compile-time built-ins may use.
    ReservedNamespace {
        /// The offending identifier.
        id: PermissionId,
    },
    /// The localization table is missing a translation a descriptor
    /// references, or an extra message declaration is invalid.
    Localization(String),
}

impl fmt::Display for PermissionRegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Duplicate { id } => write!(f, "duplicate permission id '{id}'"),
            Self::KindMismatch { id, declared } => write!(
                f,
                "permission '{id}' declares kind '{}' that contradicts its namespace",
                declared.as_str()
            ),
            Self::ReservedNamespace { id } => write!(
                f,
                "permission '{id}' occupies the reserved '{BUILTIN_NAMESPACE}' namespace"
            ),
            Self::Localization(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for PermissionRegistryError {}

impl PermissionRegistryError {
    fn from_localization(error: PermissionLocalizationError) -> Self {
        Self::Localization(error.to_string())
    }
}

/// Descriptor table of the built-in permissions.
///
/// Kept as a `const` array so the whole table is compile-time data; the unit
/// tests re-validate every identifier through [`PermissionId::new`].
pub const BUILTIN_DESCRIPTORS: [PermissionDescriptor; 13] = [
    PermissionDescriptor {
        id: permission::admin::OVERVIEW,
        kind: PermissionKind::AdminArea,
        label_i18n_key: "admin.users.operatorScope.overview",
        label: "Overview",
        group: "admin_panel",
    },
    PermissionDescriptor {
        id: permission::admin::USERS,
        kind: PermissionKind::AdminArea,
        label_i18n_key: "admin.users.operatorScope.users",
        label: "Users",
        group: "admin_panel",
    },
    PermissionDescriptor {
        id: permission::admin::PROFILES,
        kind: PermissionKind::AdminArea,
        label_i18n_key: "admin.users.operatorScope.profiles",
        label: "Profiles",
        group: "admin_panel",
    },
    PermissionDescriptor {
        id: permission::admin::TEXTURE_LIBRARY,
        kind: PermissionKind::AdminArea,
        label_i18n_key: "admin.users.operatorScope.texture_library",
        label: "Texture library",
        group: "admin_panel",
    },
    PermissionDescriptor {
        id: permission::admin::AUDIT,
        kind: PermissionKind::AdminArea,
        label_i18n_key: "admin.users.operatorScope.audit",
        label: "Audit",
        group: "admin_panel",
    },
    PermissionDescriptor {
        id: permission::admin::TASKS,
        kind: PermissionKind::AdminArea,
        label_i18n_key: "admin.users.operatorScope.tasks",
        label: "Tasks",
        group: "admin_panel",
    },
    PermissionDescriptor {
        id: permission::admin::SETTINGS,
        kind: PermissionKind::AdminArea,
        label_i18n_key: "admin.users.operatorScope.settings",
        label: "Settings",
        group: "admin_panel",
    },
    PermissionDescriptor {
        id: permission::admin::EXTERNAL_AUTH,
        kind: PermissionKind::AdminArea,
        label_i18n_key: "admin.users.operatorScope.external_auth",
        label: "External auth",
        group: "admin_panel",
    },
    PermissionDescriptor {
        id: capability::yggdrasil::ACCESS,
        kind: PermissionKind::Capability,
        label_i18n_key: "admin.users.bans.scope.yggdrasil_access",
        label: "Yggdrasil access",
        group: "yggdrasil",
    },
    PermissionDescriptor {
        id: capability::yggdrasil::JOIN,
        kind: PermissionKind::Capability,
        label_i18n_key: "admin.users.bans.scope.yggdrasil_join",
        label: "Multiplayer join",
        group: "yggdrasil",
    },
    PermissionDescriptor {
        id: capability::profile::MANAGE,
        kind: PermissionKind::Capability,
        label_i18n_key: "admin.users.bans.scope.minecraft_profile_manage",
        label: "Profile management",
        group: "profile",
    },
    PermissionDescriptor {
        id: capability::texture::UPLOAD,
        kind: PermissionKind::Capability,
        label_i18n_key: "admin.users.bans.scope.texture_upload",
        label: "Texture upload",
        group: "texture",
    },
    PermissionDescriptor {
        id: capability::texture_library::INTERACT,
        kind: PermissionKind::Capability,
        label_i18n_key: "admin.users.bans.scope.texture_library_interact",
        label: "Texture library interaction",
        group: "texture_library",
    },
];

/// Caller-owned registry of valid permission descriptors.
///
/// Built-in permissions are always present; [`PermissionRegistry::new`]
/// accepts additional descriptors so a future plugin system can register its
/// own namespaces without reshaping the call sites. Registry instances are
/// cheap to clone and hold no global state.
#[derive(Debug, Clone)]
pub struct PermissionRegistry {
    entries: BTreeMap<PermissionId, PermissionDescriptor>,
    localization: PermissionLocalization,
}

impl PermissionRegistry {
    /// Builds a registry from the built-in table plus `extra` descriptors
    /// and their translations, rejecting duplicates, kind mismatches, extras
    /// occupying the reserved built-in namespace, and any descriptor whose
    /// label key has no translation.
    ///
    /// Extra message declarations merge into the built-in translation table;
    /// redeclaring an existing message id fails so external sources cannot
    /// override built-in labels.
    ///
    /// # Errors
    ///
    /// Returns [`PermissionRegistryError`] on the first invalid descriptor or
    /// message declaration.
    pub fn new(
        extra: Vec<PermissionDescriptor>,
        extra_messages: &[PermissionLocalizationMessage<'_>],
    ) -> Result<Self, PermissionRegistryError> {
        let mut entries = BTreeMap::new();
        for descriptor in BUILTIN_DESCRIPTORS {
            insert_descriptor(&mut entries, descriptor)?;
        }
        for descriptor in extra {
            if descriptor.id.is_builtin() {
                return Err(PermissionRegistryError::ReservedNamespace { id: descriptor.id });
            }
            insert_descriptor(&mut entries, descriptor)?;
        }

        // One merged translation table: built-ins first, extras appended. A
        // duplicate message id fails the build, so external sources cannot
        // override built-in labels.
        let combined: Vec<&PermissionLocalizationMessage> = BUILTIN_PERMISSION_MESSAGES
            .iter()
            .chain(extra_messages.iter())
            .collect();
        let localization = PermissionLocalization::from_messages(
            super::permission_localization::PERMISSION_DEFAULT_LOCALE,
            combined.iter().copied(),
        )
        .map_err(PermissionRegistryError::from_localization)?;

        let referenced: Vec<&str> = entries
            .values()
            .map(|descriptor| descriptor.label_i18n_key)
            .collect();
        localization
            .validate_message_ids(referenced.iter().copied())
            .map_err(PermissionRegistryError::from_localization)?;
        Ok(Self {
            entries,
            localization,
        })
    }

    /// Registry holding exactly the built-in permissions and translations.
    ///
    /// # Errors
    ///
    /// Returns the registry error when the compile-time built-in table
    /// violates registry invariants; the unit tests keep the table valid.
    pub fn builtin() -> Result<Self, PermissionRegistryError> {
        Self::new(Vec::new(), &[])
    }

    /// Backend-owned translations backing the registered label keys.
    #[must_use]
    pub fn localization(&self) -> &PermissionLocalization {
        &self.localization
    }

    /// Descriptor registered under `id`, if any.
    #[must_use]
    pub fn get(&self, id: &PermissionId) -> Option<&PermissionDescriptor> {
        self.entries.get(id)
    }

    /// Whether `id` is registered.
    #[must_use]
    pub fn contains(&self, id: &PermissionId) -> bool {
        self.entries.contains_key(id)
    }

    /// All registered descriptors ordered by identifier.
    pub fn all(&self) -> impl Iterator<Item = &PermissionDescriptor> {
        self.entries.values()
    }

    /// Descriptors of one semantic kind ordered by identifier.
    pub fn by_kind(&self, kind: PermissionKind) -> impl Iterator<Item = &PermissionDescriptor> {
        self.entries
            .values()
            .filter(move |descriptor| descriptor.kind == kind)
    }

    /// Number of registered descriptors.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no descriptor is registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Inserts one descriptor after checking kind agreement and duplicates.
fn insert_descriptor(
    entries: &mut BTreeMap<PermissionId, PermissionDescriptor>,
    descriptor: PermissionDescriptor,
) -> Result<(), PermissionRegistryError> {
    let actual_kind = descriptor.id.kind();
    if actual_kind != descriptor.kind {
        return Err(PermissionRegistryError::KindMismatch {
            id: descriptor.id,
            declared: descriptor.kind,
        });
    }
    let id = descriptor.id.clone();
    if entries.contains_key(&id) {
        return Err(PermissionRegistryError::Duplicate { id });
    }
    entries.insert(id, descriptor);
    Ok(())
}

/// Built-in admin-area permission identifiers
/// (`com.astercosm.yggdrasil.permission.admin.*`).
///
/// These are grant-scoped panel areas assigned to operator accounts; full
/// admins pass every gate without holding any of them.
#[expect(
    clippy::module_inception,
    reason = "the identifier trees are permission::admin.* and capability.* by design; \
              the outer module owns the shared newtype, kinds, and registry"
)]
pub mod permission {
    /// Admin-panel area permissions.
    pub mod admin {
        use super::super::PermissionId;

        /// Dashboard and server overview pages.
        pub const OVERVIEW: PermissionId =
            PermissionId::builtin("com.astercosm.yggdrasil.permission.admin.overview");
        /// User account management.
        pub const USERS: PermissionId =
            PermissionId::builtin("com.astercosm.yggdrasil.permission.admin.users");
        /// Minecraft profile management.
        pub const PROFILES: PermissionId =
            PermissionId::builtin("com.astercosm.yggdrasil.permission.admin.profiles");
        /// Texture library moderation.
        pub const TEXTURE_LIBRARY: PermissionId =
            PermissionId::builtin("com.astercosm.yggdrasil.permission.admin.texture_library");
        /// Audit log access.
        pub const AUDIT: PermissionId =
            PermissionId::builtin("com.astercosm.yggdrasil.permission.admin.audit");
        /// Background task management.
        pub const TASKS: PermissionId =
            PermissionId::builtin("com.astercosm.yggdrasil.permission.admin.tasks");
        /// Runtime and system configuration.
        pub const SETTINGS: PermissionId =
            PermissionId::builtin("com.astercosm.yggdrasil.permission.admin.settings");
        /// External authentication provider management.
        pub const EXTERNAL_AUTH: PermissionId =
            PermissionId::builtin("com.astercosm.yggdrasil.permission.admin.external_auth");
    }
}

/// Built-in player capability identifiers
/// (`com.astercosm.yggdrasil.capability.*`).
///
/// These are deny-scoped abilities targeted by user bans.
pub mod capability {
    /// Yggdrasil protocol capabilities.
    pub mod yggdrasil {
        use super::super::PermissionId;

        /// Blocks Yggdrasil authenticate/refresh/token operations.
        pub const ACCESS: PermissionId =
            PermissionId::builtin("com.astercosm.yggdrasil.capability.yggdrasil.access");
        /// Blocks the multiplayer join handshake for this account.
        pub const JOIN: PermissionId =
            PermissionId::builtin("com.astercosm.yggdrasil.capability.yggdrasil.join");
    }

    /// Minecraft profile capabilities.
    pub mod profile {
        use super::super::PermissionId;

        /// Blocks creating, renaming, or deleting Minecraft profiles.
        pub const MANAGE: PermissionId =
            PermissionId::builtin("com.astercosm.yggdrasil.capability.profile.manage");
    }

    /// Texture capabilities.
    pub mod texture {
        use super::super::PermissionId;

        /// Blocks uploading new textures.
        pub const UPLOAD: PermissionId =
            PermissionId::builtin("com.astercosm.yggdrasil.capability.texture.upload");
    }

    /// Shared texture library capabilities.
    pub mod texture_library {
        use super::super::PermissionId;

        /// Blocks interacting with the shared texture library.
        pub const INTERACT: PermissionId =
            PermissionId::builtin("com.astercosm.yggdrasil.capability.texture_library.interact");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_ids_are_valid() {
        for descriptor in BUILTIN_DESCRIPTORS {
            let parsed =
                PermissionId::new(descriptor.id.as_str()).expect("builtin id must be valid");
            assert_eq!(&parsed, &descriptor.id);
            assert_eq!(parsed.kind(), descriptor.kind);
            assert!(parsed.is_builtin());
            assert!(descriptor.id.as_str().len() <= PERMISSION_ID_MAX_LEN);
        }
    }

    #[test]
    fn builtin_ids_are_unique() {
        let mut ids: Vec<String> = BUILTIN_DESCRIPTORS
            .iter()
            .map(|descriptor| descriptor.id.as_str().to_string())
            .collect();
        ids.sort_unstable();
        let count = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), count, "builtin ids must be unique");
    }

    #[test]
    fn parses_builtin_namespace_variants() {
        let id = PermissionId::new("com.example.plugin.permission.admin.dashboard")
            .expect("third-party permission id must parse");
        assert_eq!(id.kind(), PermissionKind::AdminArea);
        assert!(!id.is_builtin());

        let capability = PermissionId::new("com.example.plugin.capability.rendering.export_hd")
            .expect("third-party capability id must parse");
        assert_eq!(capability.kind(), PermissionKind::Capability);
    }

    #[test]
    fn rejects_invalid_identifiers() {
        assert_eq!(PermissionId::new("").unwrap_err(), PermissionIdError::Empty);
        assert!(matches!(
            PermissionId::new("a".repeat(PERMISSION_ID_MAX_LEN + 1)).unwrap_err(),
            PermissionIdError::TooLong { .. }
        ));
        assert_eq!(
            PermissionId::new("yggdrasil_access").unwrap_err(),
            PermissionIdError::MissingNamespace
        );
        assert_eq!(
            PermissionId::new("com.example.permission.admin")
                .ok()
                .map(|id| id.as_str().to_string()),
            Some("com.example.permission.admin".to_string())
        );
        assert!(matches!(
            PermissionId::new("com.example.permission.").unwrap_err(),
            PermissionIdError::InvalidSegment { .. }
        ));
        assert!(matches!(
            PermissionId::new("com.Example.capability.render").unwrap_err(),
            PermissionIdError::InvalidSegment { .. }
        ));
        assert!(matches!(
            PermissionId::new("com.example.capability..render").unwrap_err(),
            PermissionIdError::InvalidSegment { .. }
        ));
        assert!(matches!(
            PermissionId::new("com.example.capability.1render").unwrap_err(),
            PermissionIdError::InvalidSegment { .. }
        ));
        assert!(matches!(
            PermissionId::new("com.permission.example.capability.admin").unwrap_err(),
            PermissionIdError::AmbiguousNamespace
        ));
        assert!(matches!(
            PermissionId::new("example.capability.render").unwrap_err(),
            PermissionIdError::InvalidSegment { .. }
        ));
    }

    #[test]
    fn accepts_boundary_len() {
        let id = PermissionId::new("a_.b.capability.render").expect("two-segment ns must parse");
        assert_eq!(id.kind(), PermissionKind::Capability);
    }

    #[test]
    fn builtin_registry_holds_all_builtins() {
        let registry = PermissionRegistry::builtin().expect("built-in registry is valid");
        assert_eq!(registry.len(), BUILTIN_DESCRIPTORS.len());
        for descriptor in BUILTIN_DESCRIPTORS {
            assert!(registry.contains(&descriptor.id));
            assert_eq!(registry.get(&descriptor.id), Some(&descriptor));
        }
        assert_eq!(registry.by_kind(PermissionKind::AdminArea).count(), 8);
        assert_eq!(registry.by_kind(PermissionKind::Capability).count(), 5);
    }

    #[test]
    fn registry_rejects_duplicate_ids() {
        let id =
            PermissionId::new("com.example.plugin.capability.rendering.export").expect("valid");
        let descriptor = || PermissionDescriptor {
            id: id.clone(),
            kind: PermissionKind::Capability,
            label_i18n_key: "x",
            label: "x",
            group: "x",
        };
        let error = PermissionRegistry::new(vec![descriptor(), descriptor()], &[]).unwrap_err();
        assert!(
            matches!(error, PermissionRegistryError::Duplicate { ref id } if id.as_str() == "com.example.plugin.capability.rendering.export")
        );
    }

    #[test]
    fn registry_rejects_kind_mismatch() {
        let id =
            PermissionId::new("com.example.plugin.capability.rendering.export").expect("valid");
        let error = PermissionRegistry::new(
            vec![PermissionDescriptor {
                id,
                kind: PermissionKind::AdminArea,
                label_i18n_key: "x",
                label: "x",
                group: "x",
            }],
            &[],
        )
        .unwrap_err();
        assert!(matches!(
            error,
            PermissionRegistryError::KindMismatch {
                declared: PermissionKind::AdminArea,
                ..
            }
        ));
    }

    #[test]
    fn registry_rejects_reserved_namespace() {
        let id = PermissionId::new("com.astercosm.yggdrasil.capability.sneaky.bypass")
            .expect("valid syntax");
        let error = PermissionRegistry::new(
            vec![PermissionDescriptor {
                id,
                kind: PermissionKind::Capability,
                label_i18n_key: "x",
                label: "x",
                group: "x",
            }],
            &[],
        )
        .unwrap_err();
        assert!(matches!(
            error,
            PermissionRegistryError::ReservedNamespace { .. }
        ));
    }

    #[test]
    fn error_and_kind_display_forms_render() {
        let errors: Vec<PermissionIdError> = vec![
            PermissionIdError::Empty,
            PermissionIdError::TooLong { len: 300 },
            PermissionIdError::MissingNamespace,
            PermissionIdError::AmbiguousNamespace,
            PermissionIdError::InvalidSegment {
                segment: "Bad".to_string(),
            },
        ];
        for error in errors {
            assert!(!error.to_string().is_empty());
        }

        let registry_errors: Vec<PermissionRegistryError> = vec![
            PermissionRegistryError::Duplicate {
                id: permission::admin::USERS,
            },
            PermissionRegistryError::KindMismatch {
                id: capability::texture::UPLOAD,
                declared: PermissionKind::AdminArea,
            },
            PermissionRegistryError::ReservedNamespace {
                id: capability::texture::UPLOAD,
            },
        ];
        for error in registry_errors {
            assert!(!error.to_string().is_empty());
        }

        assert_eq!(PermissionKind::AdminArea.as_str(), "admin_area");
        assert_eq!(PermissionKind::Capability.as_str(), "capability");
        assert_eq!(PermissionKind::AdminArea.to_string(), "admin_area");
        assert_eq!(PermissionKind::Capability.to_string(), "capability");
    }

    #[test]
    fn builtin_constructor_also_runs_at_runtime() {
        // `builtin` is a const fn evaluated at compile time for the built-in
        // table; calling it at runtime keeps the wrapper counted by coverage.
        let id = PermissionId::builtin("com.example.runtime.capability.render");
        assert_eq!(id.as_str(), "com.example.runtime.capability.render");
        assert_eq!(id.kind(), PermissionKind::Capability);
    }

    #[test]
    fn registry_length_helpers_report_builtins() {
        let registry = PermissionRegistry::builtin().expect("built-in registry");
        assert!(!registry.is_empty());
        assert!(registry.all().next().is_some());
    }

    #[test]
    fn lookalike_namespaces_stay_external() {
        let lookalike =
            PermissionId::new("com.astercosm.yggdrasil2.capability.export").expect("valid syntax");
        assert!(!lookalike.is_builtin());
        let registry = PermissionRegistry::new(
            vec![PermissionDescriptor {
                id: lookalike.clone(),
                kind: PermissionKind::Capability,
                label_i18n_key: "plugin.export",
                label: "Export",
                group: "plugin",
            }],
            &[crate::permission_message!(
                "plugin.export",
                "Export",
                "导出"
            )],
        )
        .expect("look-alike namespace must not be treated as reserved");
        assert!(registry.contains(&lookalike));
    }

    #[test]
    fn localization_getter_and_serde_deserialization_round_trip() {
        let registry = PermissionRegistry::builtin().expect("built-in registry");
        // The getter backs the localizations endpoint.
        assert_eq!(
            registry
                .localization()
                .bundle(
                    &super::super::permission_localization::LocaleTag::parse("en").expect("valid"),
                )
                .namespace,
            "permissions_backend",
        );

        // The serde TryFrom path is what API payloads exercise.
        let id: PermissionId =
            serde_json::from_str("\"com.astercosm.yggdrasil.capability.texture.upload\"")
                .expect("valid id deserializes");
        assert_eq!(id, super::super::permission::capability::texture::UPLOAD);
        let bad: Result<PermissionId, _> = serde_json::from_str("\"yggdrasil_access\"");
        assert!(bad.is_err(), "legacy short names must fail deserialization");
    }

    #[test]
    fn untranslated_descriptor_keys_fail_the_build() {
        let id = PermissionId::new("com.example.plugin.capability.rendering.export")
            .expect("valid syntax");
        // No translation ships for this key: the registry must refuse to
        // build rather than let users see a raw key.
        let error = PermissionRegistry::new(
            vec![PermissionDescriptor {
                id,
                kind: PermissionKind::Capability,
                label_i18n_key: "plugin.rendering.export",
                label: "Export",
                group: "plugin",
            }],
            &[],
        )
        .unwrap_err();
        assert!(matches!(error, PermissionRegistryError::Localization(_)));
        assert!(error.to_string().contains("plugin.rendering.export"));
    }

    #[test]
    fn registry_accepts_foreign_namespace() {
        let id =
            PermissionId::new("com.example.plugin.capability.rendering.export").expect("valid");
        let registry = PermissionRegistry::new(
            vec![PermissionDescriptor {
                id: id.clone(),
                kind: PermissionKind::Capability,
                label_i18n_key: "plugin.rendering.export",
                label: "Export",
                group: "plugin",
            }],
            &[crate::permission_message!(
                "plugin.rendering.export",
                "Export rendering",
                "导出渲染"
            )],
        )
        .expect("foreign descriptor with translation must register");
        assert!(registry.contains(&id));
        assert_eq!(registry.len(), BUILTIN_DESCRIPTORS.len() + 1);
    }
}
