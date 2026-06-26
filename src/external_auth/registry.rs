//! Yggdrasil-facing registry boundary over shared Forge external-auth drivers.

use std::sync::{Arc, OnceLock};

use aster_forge_external_auth::ExternalAuthProviderDriver;

use crate::errors::Result;
use crate::types::external_auth::ExternalAuthProviderKind;

use super::{
    ExternalAuthProviderDescriptor, map_external_auth_error, map_external_auth_provider_descriptor,
};

/// Registry boundary for feature-enabled external authentication provider drivers.
pub struct ExternalAuthProviderRegistry {
    inner: aster_forge_external_auth::ExternalAuthProviderRegistry,
}

impl ExternalAuthProviderRegistry {
    /// Creates a registry populated with all Forge drivers enabled for Yggdrasil.
    pub fn new() -> Self {
        Self {
            inner: aster_forge_external_auth::ExternalAuthProviderRegistry::new(),
        }
    }

    /// Returns registered provider kinds.
    pub fn supported_kinds(&self) -> impl Iterator<Item = ExternalAuthProviderKind> + '_ {
        self.inner.supported_kinds().map(Into::into)
    }

    /// Returns registered provider descriptors sorted by provider kind.
    pub fn descriptors(&self) -> Vec<ExternalAuthProviderDescriptor> {
        self.inner
            .descriptors()
            .into_iter()
            .map(map_external_auth_provider_descriptor)
            .collect()
    }

    /// Returns whether a provider kind is registered.
    pub fn contains(&self, kind: ExternalAuthProviderKind) -> bool {
        self.inner.contains(kind.into())
    }

    /// Returns a provider descriptor by provider kind.
    pub fn descriptor_for(
        &self,
        kind: ExternalAuthProviderKind,
    ) -> Result<ExternalAuthProviderDescriptor> {
        self.inner
            .descriptor_for(kind.into())
            .map(map_external_auth_provider_descriptor)
            .map_err(map_external_auth_error)
    }

    /// Returns a registered Forge provider driver by provider kind.
    pub fn get_driver(
        &self,
        kind: ExternalAuthProviderKind,
    ) -> Result<Arc<dyn ExternalAuthProviderDriver>> {
        self.inner
            .get_driver(kind.into())
            .map_err(map_external_auth_error)
    }

    /// Returns the registered OIDC provider driver.
    pub fn oidc(&self) -> Result<Arc<dyn ExternalAuthProviderDriver>> {
        self.get_driver(ExternalAuthProviderKind::Oidc)
    }
}

impl Default for ExternalAuthProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Returns the process-wide default external-auth provider registry.
pub fn default_registry() -> &'static ExternalAuthProviderRegistry {
    static REGISTRY: OnceLock<ExternalAuthProviderRegistry> = OnceLock::new();
    REGISTRY.get_or_init(ExternalAuthProviderRegistry::new)
}
