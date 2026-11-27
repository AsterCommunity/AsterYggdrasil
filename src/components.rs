//! Shared runtime component identity constants.
//!
//! Component names are part of the shutdown-graph and health-report contract
//! between modules: the database, mail outbox, and task runtime all order
//! themselves against these identities. They live in a dependency-free leaf
//! module so infrastructure code (for example the database component) can
//! reference them without importing the runtime assembly.

/// Independent component identity for the per-instance queue consumer group.
///
/// Process-owned maintenance (metrics updates, config reload) keeps the default
/// [`aster_forge_tasks::BACKGROUND_TASKS_COMPONENT`] name, so shutdown graphs can
/// order the consumer group against it explicitly.
pub const TASK_CONSUMERS_COMPONENT: &str = "task_consumers";

/// Independent component identity for the leased scheduled maintenance group.
///
/// Unlike [`TASK_CONSUMERS_COMPONENT`], only the instance holding the runtime
/// lease runs these tasks; the component exists so health reports and shutdown
/// graphs can address the group separately from queue consumers.
pub const SCHEDULED_TASKS_COMPONENT: &str = "scheduled_tasks";
