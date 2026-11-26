//! Runtime startup assembly.

/// Startup phases shared by every serve role.
pub mod common;

#[cfg(test)]
mod tests {
    use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};

    use crate::runtime::test_support;

    #[tokio::test]
    async fn record_server_start_writes_audit_log() {
        let state = test_support::setup_state().await;
        let db = state.db_handles.reader().clone();

        crate::services::ops::audit::runtime::record_server_start(&state).await;
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        state
            .audit_writer()
            .flush(deadline)
            .await
            .expect("audit writer flush should succeed");

        let count = aster_yggdrasil_model::entities::audit_log::Entity::find()
            .filter(
                aster_yggdrasil_model::entities::audit_log::Column::Action
                    .eq(aster_yggdrasil_model::types::audit::AuditAction::ServerStart),
            )
            .count(&db)
            .await
            .expect("audit log query should succeed");
        assert_eq!(count, 1);
    }
}
