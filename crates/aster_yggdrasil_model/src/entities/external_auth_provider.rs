//! SeaORM entity definition for `external_auth_providers`.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt;
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

use crate::types::{
    external_auth::ExternalAuthProtocol, external_auth::ExternalAuthProviderKind,
    external_auth::StoredExternalAuthProviderOptions,
};

#[derive(Clone, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(table_name = "external_auth_providers")]
/// Administrator-defined external authentication provider (OIDC or OAuth2).
pub struct Model {
    /// Auto-incrementing primary key.
    #[sea_orm(primary_key)]
    pub id: i64,
    /// URL-safe identifier used in routes and references; unique per provider kind.
    pub key: String,
    /// Name shown on the sign-in button.
    pub display_name: String,
    /// Logo shown next to the provider name on sign-in pages.
    pub icon_url: Option<String>,
    /// Provider family with built-in driver support (`oidc`, `generic_oauth2`, `github`,
    /// `google`, `microsoft`, `qq`).
    pub provider_kind: ExternalAuthProviderKind,
    /// Protocol dialect used to talk to the provider (`oidc` or `oauth2`).
    pub protocol: ExternalAuthProtocol,
    /// Provider-kind specific options (for example the Microsoft tenant) stored as JSON text;
    /// redacted from debug output.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub options: StoredExternalAuthProviderOptions,
    /// OIDC issuer used for discovery; `None` for providers configured by explicit endpoints.
    pub issuer_url: Option<String>,
    /// Explicit authorization endpoint override; `None` to rely on discovery or provider defaults.
    pub authorization_url: Option<String>,
    /// Explicit token endpoint override; `None` to rely on discovery or provider defaults.
    pub token_url: Option<String>,
    /// Explicit userinfo endpoint override; `None` to rely on discovery or provider defaults.
    pub userinfo_url: Option<String>,
    /// OAuth client id registered at the provider.
    pub client_id: String,
    /// OAuth client secret; stored for server-side token exchange, never serialized
    /// to clients and redacted from debug output.
    #[serde(skip_serializing)]
    pub client_secret: Option<String>,
    /// Space-separated scope list requested during authorization.
    pub scopes: String,
    /// Whether the provider is offered on sign-in pages.
    pub enabled: bool,
    /// Whether a first-time sign-in creates a local account automatically.
    pub auto_provision_enabled: bool,
    /// Whether a first-time sign-in with an email matching a verified local account
    /// links to that account automatically.
    pub auto_link_verified_email_enabled: bool,
    /// Whether sign-in is refused when the provider does not report the email as verified.
    pub require_email_verified: bool,
    /// Claim name holding the stable subject id; `None` uses the protocol default.
    pub subject_claim: Option<String>,
    /// Claim name mapped to the local username suggestion; `None` uses the default.
    pub username_claim: Option<String>,
    /// Claim name mapped to the display name; `None` uses the default.
    pub display_name_claim: Option<String>,
    /// Claim name mapped to the email; `None` uses the default.
    pub email_claim: Option<String>,
    /// Claim name reporting email verification; `None` uses the default.
    pub email_verified_claim: Option<String>,
    /// Claim name holding group memberships; `None` skips group extraction.
    pub groups_claim: Option<String>,
    /// Claim name holding the avatar URL; `None` skips avatar extraction.
    pub avatar_url_claim: Option<String>,
    /// Email domains allowed to sign in through this provider, stored as a JSON array
    /// string; `None` or empty allows all domains.
    pub allowed_domains: Option<String>,
    /// When the provider configuration was created.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: DateTimeUtc,
    /// When the provider configuration was last modified.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: DateTimeUtc,
}

impl fmt::Debug for Model {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Model")
            .field("id", &self.id)
            .field("key", &self.key)
            .field("display_name", &self.display_name)
            .field("icon_url", &self.icon_url)
            .field("provider_kind", &self.provider_kind)
            .field("protocol", &self.protocol)
            .field("options", &"***REDACTED***")
            .field("issuer_url", &self.issuer_url)
            .field("authorization_url", &self.authorization_url)
            .field("token_url", &self.token_url)
            .field("userinfo_url", &self.userinfo_url)
            .field("client_id", &self.client_id)
            .field(
                "client_secret",
                &self.client_secret.as_ref().map(|_| "***REDACTED***"),
            )
            .field("scopes", &self.scopes)
            .field("enabled", &self.enabled)
            .field("auto_provision_enabled", &self.auto_provision_enabled)
            .field(
                "auto_link_verified_email_enabled",
                &self.auto_link_verified_email_enabled,
            )
            .field("require_email_verified", &self.require_email_verified)
            .field("subject_claim", &self.subject_claim)
            .field("username_claim", &self.username_claim)
            .field("display_name_claim", &self.display_name_claim)
            .field("email_claim", &self.email_claim)
            .field("email_verified_claim", &self.email_verified_claim)
            .field("groups_claim", &self.groups_claim)
            .field("avatar_url_claim", &self.avatar_url_claim)
            .field("allowed_domains", &self.allowed_domains)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// Relations from a provider to the flows and identities it created.
pub enum Relation {
    /// Email verification flows started through this provider.
    #[sea_orm(has_many = "super::external_auth_email_verification_flow::Entity")]
    ExternalAuthEmailVerificationFlows,
    /// External identities linked through this provider.
    #[sea_orm(has_many = "super::external_auth_identity::Entity")]
    ExternalAuthIdentities,
    /// Login flows currently or previously running against this provider.
    #[sea_orm(has_many = "super::external_auth_login_flow::Entity")]
    ExternalAuthLoginFlows,
}

impl Related<super::external_auth_email_verification_flow::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ExternalAuthEmailVerificationFlows.def()
    }
}

impl Related<super::external_auth_identity::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ExternalAuthIdentities.def()
    }
}

impl Related<super::external_auth_login_flow::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ExternalAuthLoginFlows.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_redacts_sensitive_options() {
        let now = chrono::Utc::now();
        let model = Model {
            id: 1,
            key: "microsoft".to_string(),
            display_name: "Microsoft".to_string(),
            icon_url: None,
            provider_kind: ExternalAuthProviderKind::Microsoft,
            protocol: ExternalAuthProtocol::Oidc,
            options: StoredExternalAuthProviderOptions(
                r#"{"microsoft":{"tenant":"secret-tenant"}}"#.to_string(),
            ),
            issuer_url: None,
            authorization_url: None,
            token_url: None,
            userinfo_url: None,
            client_id: "client-id".to_string(),
            client_secret: Some("client-secret".to_string()),
            scopes: "openid profile email".to_string(),
            enabled: true,
            auto_provision_enabled: false,
            auto_link_verified_email_enabled: false,
            require_email_verified: true,
            subject_claim: None,
            username_claim: None,
            display_name_claim: None,
            email_claim: None,
            email_verified_claim: None,
            groups_claim: None,
            avatar_url_claim: None,
            allowed_domains: None,
            created_at: now,
            updated_at: now,
        };

        let debug = format!("{model:?}");
        assert!(debug.contains(r#"options: "***REDACTED***""#));
        assert!(debug.contains(r#"client_secret: Some("***REDACTED***")"#));
        assert!(!debug.contains("secret-tenant"));
        assert!(!debug.contains("client-secret"));
    }
}
