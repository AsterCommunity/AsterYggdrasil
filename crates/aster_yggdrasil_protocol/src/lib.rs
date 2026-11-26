//! Project-agnostic Yggdrasil/authlib-injector protocol crate.
//!
//! This crate owns the protocol's wire model and error state machine:
//!
//! - [`error`]: [`YggdrasilErrorKind`] and [`YggdrasilError`], mapping protocol
//!   failures to the error name, message, and HTTP status that launchers expect.
//! - [`profile`]: profile, profile property, and user wire types.
//! - [`request`]: protocol request bodies (authenticate, refresh, join, ...).
//! - [`response`]: protocol response bodies, metadata, and Minecraft services
//!   compatibility payloads.
//! - [`textures`]: the base64-encoded `textures` property value serializer.
//! - [`validation`]: pure protocol validation rules (unsigned UUID shape,
//!   agent name, Minecraft profile name charset).
//!
//! The crate is intentionally free of database, HTTP-server, cache, and
//! application concerns: hosts assemble the wire types from their own storage
//! and policy, then serve them with any HTTP stack. Session-forwarding,
//! token lifecycle persistence, and signature key management remain product
//! infrastructure and are deliberately out of scope here.

#![forbid(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::unreachable,
        clippy::expect_used,
        clippy::panic,
        clippy::unimplemented,
        clippy::todo
    )
)]

pub mod error;
pub mod profile;
pub mod request;
pub mod response;
pub mod textures;
pub mod validation;

pub use error::{YggdrasilError, YggdrasilErrorKind};
pub use profile::{YggdrasilProfile, YggdrasilProfileProperty, YggdrasilUser};
pub use request::{
    YggdrasilAgentReq, YggdrasilAuthenticateReq, YggdrasilHasJoinedQuery, YggdrasilJoinReq,
    YggdrasilProfileQuery, YggdrasilRefreshReq, YggdrasilSignoutReq, YggdrasilTokenReq,
};
pub use response::{
    MinecraftServicesBanStatus, MinecraftServicesBannedScopes, MinecraftServicesCertificateResp,
    MinecraftServicesChatPreferences, MinecraftServicesFriendsPreferences,
    MinecraftServicesKeyPair, MinecraftServicesPathError, MinecraftServicesPlayerAttributesResp,
    MinecraftServicesPreferenceState, MinecraftServicesPrivacyBlocklistResp,
    MinecraftServicesPrivilege, MinecraftServicesPrivileges, MinecraftServicesPrivilegesResp,
    MinecraftServicesProfanityFilterPreferences, YggdrasilAuthenticateResp, YggdrasilErrorBody,
    YggdrasilMeta, YggdrasilMetaLinks, YggdrasilMetaResp, YggdrasilRefreshResp,
};
pub use textures::{
    TexturesPropertyValue, TexturesPropertyValueItem, TexturesPropertyValueMetadata,
    encode_textures_property_value,
};
pub use validation::minecraft_profile_name_is_valid;
