//! External authentication integration boundary.
//!
//! Provider runtime logic is shared through `aster_forge_external_auth`. Yggdrasil keeps only the
//! application-owned persistence, API, and error mapping boundary here.

pub mod driver;
pub mod registry;

pub use aster_forge_external_auth::{
    ExternalAuthAuthorizationStart, ExternalAuthCallback, ExternalAuthProfile,
    ExternalAuthProviderConfig, ExternalAuthProviderDriver, ExternalAuthProviderTestCheck,
    ExternalAuthProviderTestResult,
};

pub use crate::types::{ExternalAuthProtocol, ExternalAuthProviderKind};
pub use driver::external_auth_provider_config_from_model;
pub use registry::ExternalAuthProviderRegistry;

/// Yggdrasil-facing provider descriptor using app-owned provider kind and protocol enums.
#[derive(Clone, Debug)]
pub struct ExternalAuthProviderDescriptor {
    pub kind: ExternalAuthProviderKind,
    pub protocol: ExternalAuthProtocol,
    pub display_name: &'static str,
    pub description: &'static str,
    pub default_scopes: &'static str,
    pub issuer_url_required: bool,
    pub manual_endpoint_configuration_supported: bool,
    pub authorization_url_required: bool,
    pub token_url_required: bool,
    pub userinfo_url_required: bool,
    pub supports_discovery: bool,
    pub supports_pkce: bool,
    pub supports_email_verified_claim: bool,
}

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
