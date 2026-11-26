//! External authentication integration boundary.
//!
//! Provider runtime logic is shared through `aster_forge_external_auth`. Yggdrasil keeps only the
//! application-owned persistence, API, and error mapping boundary here.

pub mod driver;
pub mod registry;

pub use aster_yggdrasil_model::types::{
    external_auth::ExternalAuthProtocol, external_auth::ExternalAuthProviderKind,
};
pub use driver::external_auth_provider_config_from_model;
pub use registry::ExternalAuthProviderRegistry;

/// Yggdrasil-facing provider descriptor using app-owned provider kind and protocol enums.
#[derive(Clone, Debug)]
pub struct ExternalAuthProviderDescriptor {
    /// Provider kind this descriptor describes.
    pub kind: ExternalAuthProviderKind,
    /// Underlying protocol (OIDC or generic OAuth 2.0).
    pub protocol: ExternalAuthProtocol,
    /// Human-readable kind name for admin UI.
    pub display_name: &'static str,
    /// Short explanation of the kind shown in admin UI.
    pub description: &'static str,
    /// Space-separated OAuth scopes applied when the admin leaves scopes empty.
    pub default_scopes: &'static str,
    /// Whether `issuer_url` must be configured for this kind.
    pub issuer_url_required: bool,
    /// Whether authorization/token/userinfo endpoints can be set manually
    /// instead of being derived from discovery or fixed per kind.
    pub manual_endpoint_configuration_supported: bool,
    /// Whether `authorization_url` must be configured for this kind.
    pub authorization_url_required: bool,
    /// Whether `token_url` must be configured for this kind.
    pub token_url_required: bool,
    /// Whether `userinfo_url` must be configured for this kind.
    pub userinfo_url_required: bool,
    /// Whether the driver performs OIDC discovery against `issuer_url`.
    pub supports_discovery: bool,
    /// Whether the driver uses PKCE for the authorization code flow.
    pub supports_pkce: bool,
    /// Whether the provider exposes an email-verified claim that can be
    /// honored by `require_email_verified`.
    pub supports_email_verified_claim: bool,
}

/// Converts a Forge driver descriptor into the app-owned descriptor type so
/// route/service code never touches Forge enums directly.
pub(crate) fn map_external_auth_provider_descriptor(
    descriptor: aster_forge_external_auth::ExternalAuthProviderDescriptor,
) -> ExternalAuthProviderDescriptor {
    ExternalAuthProviderDescriptor {
        kind: descriptor.kind.into(),
        protocol: descriptor.protocol.into(),
        display_name: descriptor.display_name,
        description: descriptor.description,
        default_scopes: descriptor.default_scopes,
        issuer_url_required: descriptor.issuer_url_required,
        manual_endpoint_configuration_supported: descriptor.manual_endpoint_configuration_supported,
        authorization_url_required: descriptor.authorization_url_required,
        token_url_required: descriptor.token_url_required,
        userinfo_url_required: descriptor.userinfo_url_required,
        supports_discovery: descriptor.supports_discovery,
        supports_pkce: descriptor.supports_pkce,
        supports_email_verified_claim: descriptor.supports_email_verified_claim,
    }
}

/// Converts a Forge external-auth error into the app error taxonomy.
pub(crate) fn map_external_auth_error(
    error: aster_forge_external_auth::ExternalAuthError,
) -> crate::errors::AsterError {
    error.into()
}

/// Maps a Forge external-auth result into Yggdrasil's application error type.
pub(crate) trait MapExternalAuthResult<T> {
    fn map_external_auth(self) -> crate::errors::Result<T>;
}

impl<T> MapExternalAuthResult<T> for aster_forge_external_auth::Result<T> {
    fn map_external_auth(self) -> crate::errors::Result<T> {
        self.map_err(map_external_auth_error)
    }
}
