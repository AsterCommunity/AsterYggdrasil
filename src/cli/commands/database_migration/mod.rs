//! Offline cross-database migration command: moves instance data between
//! SQLite, PostgreSQL, and MySQL backends with a resumable checkpoint.
//!
//! Scope: the database only. Texture objects in `texture_storage` are not
//! touched by this command; keeping them reachable under the new deployment
//! is the operator's responsibility.

pub(in crate::cli) mod apply;
pub(in crate::cli) mod args;
pub(in crate::cli) mod checkpoint;
pub(in crate::cli) mod execute;
pub(in crate::cli) mod model;
pub(in crate::cli) mod plan;
pub(in crate::cli) mod render;
pub(in crate::cli) mod schema;
pub(in crate::cli) mod verify;

pub(in crate::cli) use args::DatabaseMigrateArgs;
pub(in crate::cli) use execute::execute_database_migration;
