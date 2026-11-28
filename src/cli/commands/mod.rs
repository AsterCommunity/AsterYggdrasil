//! Operational CLI subcommands.

#[cfg(feature = "operations-cli")]
pub(in crate::cli) mod database_migration;
#[cfg(feature = "operations-cli")]
pub(in crate::cli) mod hash_password;
pub(in crate::cli) mod version;
