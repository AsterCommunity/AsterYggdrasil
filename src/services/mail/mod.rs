//! Mail domain: audit trail, persisted outbox dispatch, sending, and templates.

/// Audit trail for outbound mail (send and delivery-failure records).
pub mod audit;
pub mod outbox;
pub mod outbox_runtime;
pub mod sender;
pub mod template;
