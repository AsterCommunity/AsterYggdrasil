//! Stable root exports for shared domain types.
//!
//! `crate::types` is the compatibility facade used across entities,
//! repositories, services, API DTOs, and tests. Put new domain types in a
//! concrete submodule first; add root exports only when the type is intentionally
//! shared across module boundaries.

pub use super::audit::{AuditAction, AuditEntityType};
pub use super::auth::{TokenType, VerificationChannel, VerificationPurpose};
pub use super::config::{SystemConfigSource, SystemConfigValueType, SystemConfigVisibility};
pub use super::external_auth::{
    ExternalAuthKind, ExternalAuthProtocol, ExternalAuthProviderKind, ExternalAuthProviderOptions,
    MicrosoftExternalAuthProviderOptions, StoredExternalAuthProviderOptions,
    parse_external_auth_provider_options, serialize_external_auth_provider_options,
};
pub use super::mail::{MailOutboxStatus, MailTemplateCode, StoredMailPayload};
pub use super::passkey::StoredPasskeyCredential;
pub use super::patch::{NullablePatch, deserialize_nullable_patch_option};
pub use super::task::{
    BackgroundTaskKind, BackgroundTaskStatus, StoredTaskPayload, StoredTaskResult,
    StoredTaskRuntime, StoredTaskSteps,
};
pub use super::user::{AvatarSource, UserInvitationStatus, UserRole, UserStatus};
pub use super::yggdrasil::{
    MinecraftTextureModel, MinecraftTextureType, MinecraftTextureVisibility,
};
