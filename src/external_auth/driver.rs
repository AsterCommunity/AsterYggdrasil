//! Application boundary helpers for shared external authentication drivers.
//!
//! Runtime provider behavior lives in `aster_forge_external_auth`. This module keeps the
//! Yggdrasil-owned persistence boundary: SeaORM models and active enums are converted into Forge
//! runtime DTOs immediately before a provider driver is invoked.

use crate::entities::external_auth_provider;
use crate::types::parse_external_auth_provider_options;

/// Builds the Forge runtime provider config from a stored Yggdrasil provider row.
pub fn external_auth_provider_config_from_model(
    provider: &external_auth_provider::Model,
) -> aster_forge_external_auth::ExternalAuthProviderConfig {
    aster_forge_external_auth::ExternalAuthProviderConfig {
        id: provider.id,
        key: provider.key.clone(),
        provider_kind: provider.provider_kind.into(),
        protocol: provider.protocol.into(),
        options: parse_external_auth_provider_options(provider.options.as_ref()).into(),
        issuer_url: provider.issuer_url.clone(),
        authorization_url: provider.authorization_url.clone(),
        token_url: provider.token_url.clone(),
        userinfo_url: provider.userinfo_url.clone(),
        client_id: provider.client_id.clone(),
        client_secret: provider.client_secret.clone(),
        scopes: provider.scopes.clone(),
        subject_claim: provider.subject_claim.clone(),
        username_claim: provider.username_claim.clone(),
        display_name_claim: provider.display_name_claim.clone(),
        email_claim: provider.email_claim.clone(),
        email_verified_claim: provider.email_verified_claim.clone(),
        groups_claim: provider.groups_claim.clone(),
        avatar_url_claim: provider.avatar_url_claim.clone(),
    }
}
