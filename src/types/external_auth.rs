use sea_orm::{DeriveValueType, entity::prelude::*};
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter, DeriveActiveEnum, Serialize, Deserialize,
)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
#[serde(rename_all = "snake_case")]
pub enum ExternalAuthProviderKind {
    #[sea_orm(string_value = "oidc")]
    Oidc,
    #[serde(rename = "generic_oauth2", alias = "oauth2")]
    #[sea_orm(string_value = "generic_oauth2")]
    GenericOAuth2,
    #[serde(rename = "github")]
    #[sea_orm(string_value = "github")]
    GitHub,
    #[serde(rename = "google")]
    #[sea_orm(string_value = "google")]
    Google,
    #[serde(rename = "microsoft")]
    #[sea_orm(string_value = "microsoft")]
    Microsoft,
    #[serde(rename = "qq")]
    #[sea_orm(string_value = "qq")]
    Qq,
}

pub type ExternalAuthKind = ExternalAuthProviderKind;

impl ExternalAuthProviderKind {
    pub const ALL: [Self; 6] = [
        Self::Oidc,
        Self::GenericOAuth2,
        Self::GitHub,
        Self::Google,
        Self::Microsoft,
        Self::Qq,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Oidc => "oidc",
            Self::GenericOAuth2 => "generic_oauth2",
            Self::GitHub => "github",
            Self::Google => "google",
            Self::Microsoft => "microsoft",
            Self::Qq => "qq",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "oidc" => Some(Self::Oidc),
            "oauth2" | "generic_oauth2" => Some(Self::GenericOAuth2),
            "github" => Some(Self::GitHub),
            "google" => Some(Self::Google),
            "microsoft" => Some(Self::Microsoft),
            "qq" => Some(Self::Qq),
            _ => None,
        }
    }

    pub fn default_protocol(self) -> ExternalAuthProtocol {
        match self {
            Self::Oidc => ExternalAuthProtocol::Oidc,
            Self::GenericOAuth2 | Self::GitHub | Self::Qq => ExternalAuthProtocol::OAuth2,
            Self::Google | Self::Microsoft => ExternalAuthProtocol::Oidc,
        }
    }
}

impl std::fmt::Display for ExternalAuthProviderKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl From<ExternalAuthProviderKind> for aster_forge_external_auth::ExternalAuthProviderKind {
    fn from(value: ExternalAuthProviderKind) -> Self {
        match value {
            ExternalAuthProviderKind::Oidc => Self::Oidc,
            ExternalAuthProviderKind::GenericOAuth2 => Self::GenericOAuth2,
            ExternalAuthProviderKind::GitHub => Self::GitHub,
            ExternalAuthProviderKind::Google => Self::Google,
            ExternalAuthProviderKind::Microsoft => Self::Microsoft,
            ExternalAuthProviderKind::Qq => Self::Qq,
        }
    }
}

impl From<aster_forge_external_auth::ExternalAuthProviderKind> for ExternalAuthProviderKind {
    fn from(value: aster_forge_external_auth::ExternalAuthProviderKind) -> Self {
        match value {
            aster_forge_external_auth::ExternalAuthProviderKind::Oidc => Self::Oidc,
            aster_forge_external_auth::ExternalAuthProviderKind::GenericOAuth2 => {
                Self::GenericOAuth2
            }
            aster_forge_external_auth::ExternalAuthProviderKind::GitHub => Self::GitHub,
            aster_forge_external_auth::ExternalAuthProviderKind::Google => Self::Google,
            aster_forge_external_auth::ExternalAuthProviderKind::Microsoft => Self::Microsoft,
            aster_forge_external_auth::ExternalAuthProviderKind::Qq => Self::Qq,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
#[serde(rename_all = "snake_case")]
pub enum ExternalAuthProtocol {
    #[sea_orm(string_value = "oidc")]
    Oidc,
    #[serde(rename = "oauth2")]
    #[sea_orm(string_value = "oauth2")]
    OAuth2,
}

impl ExternalAuthProtocol {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Oidc => "oidc",
            Self::OAuth2 => "oauth2",
        }
    }
}

impl From<ExternalAuthProtocol> for aster_forge_external_auth::ExternalAuthProtocol {
    fn from(value: ExternalAuthProtocol) -> Self {
        match value {
            ExternalAuthProtocol::Oidc => Self::Oidc,
            ExternalAuthProtocol::OAuth2 => Self::OAuth2,
        }
    }
}

impl From<aster_forge_external_auth::ExternalAuthProtocol> for ExternalAuthProtocol {
    fn from(value: aster_forge_external_auth::ExternalAuthProtocol) -> Self {
        match value {
            aster_forge_external_auth::ExternalAuthProtocol::Oidc => Self::Oidc,
            aster_forge_external_auth::ExternalAuthProtocol::OAuth2 => Self::OAuth2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, DeriveValueType)]
pub struct StoredExternalAuthProviderOptions(pub String);

impl StoredExternalAuthProviderOptions {
    pub const EMPTY_JSON: &str = "{}";

    pub fn empty() -> Self {
        Self(Self::EMPTY_JSON.to_string())
    }
}

impl AsRef<str> for StoredExternalAuthProviderOptions {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for StoredExternalAuthProviderOptions {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<StoredExternalAuthProviderOptions> for String {
    fn from(value: StoredExternalAuthProviderOptions) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct ExternalAuthProviderOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub microsoft: Option<MicrosoftExternalAuthProviderOptions>,
}

impl ExternalAuthProviderOptions {
    pub fn normalized(mut self) -> Self {
        if let Some(microsoft) = self.microsoft.take() {
            self.microsoft = microsoft.normalized();
        }
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct MicrosoftExternalAuthProviderOptions {
    pub tenant: String,
}

impl MicrosoftExternalAuthProviderOptions {
    pub fn new(tenant: impl Into<String>) -> Self {
        Self {
            tenant: tenant.into(),
        }
    }

    fn normalized(self) -> Option<Self> {
        let tenant = self.tenant.trim().to_string();
        (!tenant.is_empty()).then_some(Self { tenant })
    }
}

impl From<MicrosoftExternalAuthProviderOptions>
    for aster_forge_external_auth::MicrosoftExternalAuthProviderOptions
{
    fn from(value: MicrosoftExternalAuthProviderOptions) -> Self {
        Self::new(value.tenant)
    }
}

impl From<aster_forge_external_auth::MicrosoftExternalAuthProviderOptions>
    for MicrosoftExternalAuthProviderOptions
{
    fn from(value: aster_forge_external_auth::MicrosoftExternalAuthProviderOptions) -> Self {
        Self::new(value.tenant)
    }
}

impl From<ExternalAuthProviderOptions> for aster_forge_external_auth::ExternalAuthProviderOptions {
    fn from(value: ExternalAuthProviderOptions) -> Self {
        Self {
            microsoft: value.microsoft.map(Into::into),
        }
    }
}

impl From<aster_forge_external_auth::ExternalAuthProviderOptions> for ExternalAuthProviderOptions {
    fn from(value: aster_forge_external_auth::ExternalAuthProviderOptions) -> Self {
        Self {
            microsoft: value.microsoft.map(Into::into),
        }
    }
}

pub fn parse_external_auth_provider_options(options: &str) -> ExternalAuthProviderOptions {
    serde_json::from_str::<ExternalAuthProviderOptions>(options)
        .unwrap_or_else(|error| {
            if !options.is_empty() && options != StoredExternalAuthProviderOptions::EMPTY_JSON {
                tracing::warn!("invalid external auth provider options JSON '{options}': {error}");
            }
            ExternalAuthProviderOptions::default()
        })
        .normalized()
}

pub fn serialize_external_auth_provider_options(
    options: &ExternalAuthProviderOptions,
) -> std::result::Result<StoredExternalAuthProviderOptions, serde_json::Error> {
    serde_json::to_string(&options.clone().normalized()).map(StoredExternalAuthProviderOptions)
}
