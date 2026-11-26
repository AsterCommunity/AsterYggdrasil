//! Administrator overview aggregation.

use serde::Serialize;
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

use crate::api::response::SystemInfoResponse;
use crate::api::routes::health;
use crate::db::repository::{
    audit_log_repo, auth_session_repo, background_task_repo, minecraft_profile_repo,
    minecraft_texture_repo, user_repo, yggdrasil_token_repo,
};
use crate::errors::{AsterError, Result};
use crate::runtime::AppState;
use crate::services::ops::audit::{self, AuditLogEntry};
use crate::services::ops::task;
use crate::services::ops::task::types::{RuntimeSystemHealthStatus, RuntimeTaskResult};
use aster_yggdrasil_model::types::audit::AuditAction;
use aster_yggdrasil_model::types::task::BackgroundTaskStatus;
const RECENT_ACTIVITY_LIMIT: u64 = 6;
const ACTIVITY_TREND_DAYS: i64 = 7;
const USER_ACTIVITY_ACTIONS: &[AuditAction] = &[
    AuditAction::UserRegister,
    AuditAction::UserLogin,
    AuditAction::UserLogout,
    AuditAction::UserRefreshToken,
    AuditAction::UserRevokeSession,
    AuditAction::UserRevokeOtherSessions,
    AuditAction::UserChangePassword,
    AuditAction::UserConfirmRegistration,
    AuditAction::UserRequestEmailChange,
    AuditAction::UserResendEmailChange,
    AuditAction::UserConfirmEmailChange,
    AuditAction::UserRequestPasswordReset,
    AuditAction::UserConfirmPasswordReset,
    AuditAction::UserUpdateProfile,
    AuditAction::UserPasskeyRegister,
    AuditAction::UserPasskeyRename,
    AuditAction::UserPasskeyDelete,
    AuditAction::UserPasskeyLogin,
    AuditAction::UserExternalAuthLogin,
    AuditAction::UserExternalAuthLink,
    AuditAction::UserExternalAuthUnlink,
    AuditAction::MinecraftProfileCreate,
    AuditAction::MinecraftProfileRename,
    AuditAction::MinecraftProfileDelete,
    AuditAction::MinecraftTextureUpload,
    AuditAction::MinecraftTextureBind,
    AuditAction::MinecraftTextureDelete,
    AuditAction::YggdrasilAuthenticate,
    AuditAction::YggdrasilRefreshToken,
    AuditAction::YggdrasilInvalidateToken,
    AuditAction::YggdrasilSignout,
    AuditAction::YggdrasilJoinServer,
];
const YGGDRASIL_ACTIVITY_ACTIONS: &[AuditAction] = &[
    AuditAction::YggdrasilAuthenticate,
    AuditAction::YggdrasilRefreshToken,
    AuditAction::YggdrasilInvalidateToken,
    AuditAction::YggdrasilSignout,
    AuditAction::YggdrasilJoinServer,
];

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
/// Platform-wide counts for the admin overview cards.
pub struct AdminOverviewSummary {
    /// All registered user accounts.
    pub total_users: u64,
    /// All live Minecraft profiles.
    pub minecraft_profile_count: u64,
    /// All stored texture records.
    pub texture_count: u64,
    /// Currently unrevoked auth sessions.
    pub active_session_count: u64,
    /// Currently valid Yggdrasil access tokens.
    pub active_yggdrasil_token_count: u64,
    /// Background tasks currently being executed.
    pub processing_task_count: u64,
    /// Background tasks waiting to run, including retry-scheduled ones.
    pub pending_task_count: u64,
}

/// Verdict for one platform service shown in the overview service grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub enum AdminOverviewServiceStatusKind {
    /// Service is operating normally.
    Ok,
    /// Service needs attention (e.g. a queue is backing up).
    Warning,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
/// One platform service's status line for the admin overview service grid.
pub struct AdminOverviewServiceStatus {
    /// Stable service identifier (e.g. `database`, `background_tasks`).
    pub key: String,
    /// Verdict for this service.
    pub status: AdminOverviewServiceStatusKind,
    /// Primary measurement shown for the service, preformatted.
    pub metric: Option<String>,
    /// Secondary explanatory text, when the service has more to say.
    pub detail: Option<String>,
}

/// System health verdict shown in the overview; `Unknown` when no health
/// check has run yet or the report is missing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub enum AdminOverviewSystemHealthStatus {
    /// No health report available.
    Unknown,
    /// All checked components operational.
    Healthy,
    /// Operational with fallbacks in effect.
    Degraded,
    /// Some components not operational.
    Unhealthy,
}

impl From<RuntimeSystemHealthStatus> for AdminOverviewSystemHealthStatus {
    fn from(value: RuntimeSystemHealthStatus) -> Self {
        match value {
            RuntimeSystemHealthStatus::Healthy => Self::Healthy,
            RuntimeSystemHealthStatus::Degraded => Self::Degraded,
            RuntimeSystemHealthStatus::Unhealthy => Self::Unhealthy,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
/// One runtime component's health verdict for the overview health panel.
pub struct AdminOverviewSystemHealthComponent {
    /// Stable component identifier (e.g. `database`, `cache`).
    pub name: String,
    /// Verdict for this component.
    pub status: AdminOverviewSystemHealthStatus,
    /// Human-readable explanation of the verdict.
    pub message: String,
    /// Structured key/value details for diagnostics; omitted when empty.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub details: Vec<aster_forge_runtime::HealthComponentDetail>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
/// Latest known system health for the overview panel, sourced from the most
/// recent health-check and storage-consistency tasks rather than a live
/// probe.
pub struct AdminOverviewSystemHealthSummary {
    /// Worst verdict across all reported components.
    pub status: AdminOverviewSystemHealthStatus,
    /// Per-component verdicts.
    pub components: Vec<AdminOverviewSystemHealthComponent>,
    /// When the sourced report was produced (UTC); absent when no check has
    /// run yet.
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    pub checked_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Background task record the report came from, when one exists.
    pub task_id: Option<i64>,
    /// Report summary line, falling back to the task's status text.
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
/// One day of platform activity counts for the overview trend chart.
pub struct AdminOverviewTrendPoint {
    /// UTC calendar day this point covers, formatted `YYYY-MM-DD`.
    pub date: String,
    /// Distinct users with a user-scope audit action that day.
    pub active_users: u64,
    /// Distinct profiles with a Yggdrasil audit action that day.
    pub active_players: u64,
    /// Texture uploads recorded that day.
    pub new_textures: u64,
    /// Yggdrasil protocol audit records created that day.
    pub yggdrasil_api_calls: u64,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
/// Aggregate admin dashboard response: counters, service statuses, system
/// health, activity trend, and the latest audit entries.
pub struct AdminOverviewResp {
    /// Platform-wide counts.
    pub summary: AdminOverviewSummary,
    /// Per-service status lines for the service grid.
    pub services: Vec<AdminOverviewServiceStatus>,
    /// Latest known system health.
    pub system_health: AdminOverviewSystemHealthSummary,
    /// Daily activity counts for the trend chart.
    pub activity_trend: Vec<AdminOverviewTrendPoint>,
    /// Most recent audit entries for the activity feed.
    pub recent_activity: Vec<AuditLogEntry>,
    /// Process/system metadata (version, uptime, backend choices).
    pub system_info: SystemInfoResponse,
}

/// Aggregates everything the admin dashboard needs: platform counters,
/// per-service statuses, latest system health, the 7-day activity trend, and
/// the most recent audit entries.
pub async fn overview(state: &AppState) -> Result<AdminOverviewResp> {
    let total_users = user_repo::count_all(state.reader_db()).await?;
    let minecraft_profile_count = minecraft_profile_repo::count_all(state.reader_db()).await?;
    let texture_count = minecraft_texture_repo::count_all(state.reader_db()).await?;
    let active_session_count = auth_session_repo::count_active(state.reader_db()).await?;
    let active_yggdrasil_token_count =
        yggdrasil_token_repo::count_active(state.reader_db()).await?;
    let processing_task_count = background_task_repo::count_processing(state.reader_db()).await?;
    let pending_task_count =
        background_task_repo::count_pending_or_retry(state.reader_db()).await?;
    let recent_activity = audit::recent(
        state,
        audit::AuditLogFilters {
            user_id: None,
            action: None,
            entity_type: None,
            entity_id: None,
            after: None,
            before: None,
        },
        RECENT_ACTIVITY_LIMIT,
    )
    .await?;
    let system_health = load_system_health_summary(state).await?;
    let activity_trend = load_activity_trend(state).await?;

    let summary = AdminOverviewSummary {
        total_users,
        minecraft_profile_count,
        texture_count,
        active_session_count,
        active_yggdrasil_token_count,
        processing_task_count,
        pending_task_count,
    };

    let services = vec![
        AdminOverviewServiceStatus {
            key: "database".to_string(),
            status: AdminOverviewServiceStatusKind::Ok,
            metric: Some(format!("{total_users} users")),
            detail: None,
        },
        AdminOverviewServiceStatus {
            key: "yggdrasil".to_string(),
            status: AdminOverviewServiceStatusKind::Ok,
            metric: Some(format!("{active_yggdrasil_token_count} active tokens")),
            detail: None,
        },
        AdminOverviewServiceStatus {
            key: "session".to_string(),
            status: AdminOverviewServiceStatusKind::Ok,
            metric: Some(format!("{active_session_count} active sessions")),
            detail: None,
        },
        AdminOverviewServiceStatus {
            key: "object_storage".to_string(),
            status: AdminOverviewServiceStatusKind::Ok,
            metric: Some(state.texture_storage().backend_name().to_string()),
            detail: Some(format!("{texture_count} texture records")),
        },
        AdminOverviewServiceStatus {
            key: "background_tasks".to_string(),
            status: if pending_task_count > 0 {
                AdminOverviewServiceStatusKind::Warning
            } else {
                AdminOverviewServiceStatusKind::Ok
            },
            metric: Some(format!(
                "{processing_task_count} processing / {pending_task_count} queued"
            )),
            detail: None,
        },
    ];

    Ok(AdminOverviewResp {
        summary,
        services,
        system_health,
        activity_trend,
        recent_activity,
        system_info: health::system_info_response(state),
    })
}

/// Builds the last [`ACTIVITY_TREND_DAYS`] days of per-day activity counts
/// from audit log aggregates.
async fn load_activity_trend(state: &AppState) -> Result<Vec<AdminOverviewTrendPoint>> {
    let today = chrono::Utc::now().date_naive();
    let mut points = Vec::new();

    for day_offset in (0..ACTIVITY_TREND_DAYS).rev() {
        let day = today - chrono::Duration::days(day_offset);
        let start = day
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| AsterError::internal_error("midnight should be valid"))?
            .and_utc();
        let end = start + chrono::Duration::days(1);

        points.push(AdminOverviewTrendPoint {
            date: day.format("%Y-%m-%d").to_string(),
            active_users: audit_log_repo::count_distinct_users_created_between_with_actions(
                state.reader_db(),
                start,
                end,
                USER_ACTIVITY_ACTIONS,
            )
            .await?,
            active_players: audit_log_repo::count_distinct_users_created_between_with_actions(
                state.reader_db(),
                start,
                end,
                YGGDRASIL_ACTIVITY_ACTIONS,
            )
            .await?,
            new_textures: minecraft_texture_repo::count_created_between(
                state.reader_db(),
                start,
                end,
            )
            .await?,
            yggdrasil_api_calls: audit_log_repo::count_created_between_with_actions(
                state.reader_db(),
                start,
                end,
                YGGDRASIL_ACTIVITY_ACTIONS,
            )
            .await?,
        });
    }

    Ok(points)
}

async fn load_system_health_summary(state: &AppState) -> Result<AdminOverviewSystemHealthSummary> {
    let mut summary = load_core_system_health_summary(state).await?;
    if let Some(observation) = load_yggdrasil_storage_consistency_observation(state).await? {
        summary.status = worst_system_health_status(summary.status, observation.component.status);
        summary.components.push(observation.component);
        summary.task_id = latest_task_id(
            summary.task_id,
            summary.checked_at,
            observation.task_id,
            observation.checked_at,
        );
        summary.checked_at = latest_checked_at(summary.checked_at, observation.checked_at);
    }

    Ok(summary)
}

async fn load_core_system_health_summary(
    state: &AppState,
) -> Result<AdminOverviewSystemHealthSummary> {
    let task = task::runtime::find_latest_system_runtime_by_task_name(
        state,
        task::SystemRuntimeTaskKind::SystemHealthCheck,
    )
    .await?;
    let Some(task) = task else {
        return Ok(unknown_system_health(None, None, None));
    };

    let checked_at = task.finished_at.or(Some(task.updated_at));
    let task_id = Some(task.id);
    let fallback_summary = task.status_text.clone();
    let Some(result_json) = task.result_json.as_ref() else {
        return Ok(unknown_system_health(task_id, checked_at, fallback_summary));
    };

    let result = match serde_json::from_str::<RuntimeTaskResult>(result_json.as_ref()) {
        Ok(result) => result,
        Err(error) => {
            tracing::warn!(
                task_id = task.id,
                error = %error,
                "failed to decode latest system health task result"
            );
            return Ok(unknown_system_health(task_id, checked_at, fallback_summary));
        }
    };

    let summary = result.summary.or(fallback_summary);
    let Some(system_health) = result.system_health else {
        return Ok(unknown_system_health(task_id, checked_at, summary));
    };

    Ok(AdminOverviewSystemHealthSummary {
        status: system_health.status.into(),
        components: system_health
            .components
            .into_iter()
            .map(|component| AdminOverviewSystemHealthComponent {
                name: component.name,
                status: component.status.into(),
                message: component.message,
                details: component.details,
            })
            .collect(),
        checked_at,
        task_id,
        summary,
    })
}

#[derive(Debug, Clone)]
struct AdminOverviewHealthObservation {
    component: AdminOverviewSystemHealthComponent,
    checked_at: Option<chrono::DateTime<chrono::Utc>>,
    task_id: Option<i64>,
}

async fn load_yggdrasil_storage_consistency_observation(
    state: &AppState,
) -> Result<Option<AdminOverviewHealthObservation>> {
    let task = task::runtime::find_latest_system_runtime_by_task_name(
        state,
        task::SystemRuntimeTaskKind::YggdrasilStorageConsistencyCheck,
    )
    .await?;
    let Some(task) = task else {
        return Ok(None);
    };

    let checked_at = task.finished_at.or(Some(task.updated_at));
    let result_summary =
        task.result_json.as_ref().and_then(|result_json| {
            match serde_json::from_str::<RuntimeTaskResult>(result_json.as_ref()) {
                Ok(result) => result.summary,
                Err(error) => {
                    tracing::warn!(
                        task_id = task.id,
                        error = %error,
                        "failed to decode latest yggdrasil storage consistency task result"
                    );
                    None
                }
            }
        });
    let status = match task.status {
        BackgroundTaskStatus::Succeeded => AdminOverviewSystemHealthStatus::Healthy,
        BackgroundTaskStatus::Failed => AdminOverviewSystemHealthStatus::Unhealthy,
        BackgroundTaskStatus::Pending
        | BackgroundTaskStatus::Processing
        | BackgroundTaskStatus::Retry => AdminOverviewSystemHealthStatus::Degraded,
        BackgroundTaskStatus::Canceled => AdminOverviewSystemHealthStatus::Unknown,
    };
    let message = task
        .last_error
        .clone()
        .or_else(|| result_summary.clone())
        .or_else(|| task.status_text.clone())
        .unwrap_or_else(|| format!("Yggdrasil storage consistency task {}", task.status));

    Ok(Some(AdminOverviewHealthObservation {
        component: AdminOverviewSystemHealthComponent {
            name: "yggdrasil_storage_consistency".to_string(),
            status,
            message,
            details: Vec::new(),
        },
        checked_at,
        task_id: Some(task.id),
    }))
}

fn latest_checked_at(
    current: Option<chrono::DateTime<chrono::Utc>>,
    candidate: Option<chrono::DateTime<chrono::Utc>>,
) -> Option<chrono::DateTime<chrono::Utc>> {
    match (current, candidate) {
        (Some(current), Some(candidate)) => Some(current.max(candidate)),
        (Some(current), None) => Some(current),
        (None, Some(candidate)) => Some(candidate),
        (None, None) => None,
    }
}

fn latest_task_id(
    current_task_id: Option<i64>,
    current_checked_at: Option<chrono::DateTime<chrono::Utc>>,
    candidate_task_id: Option<i64>,
    candidate_checked_at: Option<chrono::DateTime<chrono::Utc>>,
) -> Option<i64> {
    match (
        current_task_id,
        current_checked_at,
        candidate_task_id,
        candidate_checked_at,
    ) {
        (Some(current_task_id), Some(current_at), Some(candidate_task_id), Some(candidate_at)) => {
            if candidate_at > current_at {
                Some(candidate_task_id)
            } else {
                Some(current_task_id)
            }
        }
        (None, _, Some(candidate_task_id), _) => Some(candidate_task_id),
        (Some(current_task_id), _, None, _) => Some(current_task_id),
        (Some(_current_task_id), None, Some(candidate_task_id), Some(_)) => Some(candidate_task_id),
        (current_task_id, _, _, _) => current_task_id,
    }
}

fn worst_system_health_status(
    left: AdminOverviewSystemHealthStatus,
    right: AdminOverviewSystemHealthStatus,
) -> AdminOverviewSystemHealthStatus {
    if system_health_status_rank(right) > system_health_status_rank(left) {
        right
    } else {
        left
    }
}

fn system_health_status_rank(status: AdminOverviewSystemHealthStatus) -> u8 {
    match status {
        AdminOverviewSystemHealthStatus::Healthy => 0,
        AdminOverviewSystemHealthStatus::Unknown => 1,
        AdminOverviewSystemHealthStatus::Degraded => 2,
        AdminOverviewSystemHealthStatus::Unhealthy => 3,
    }
}

fn unknown_system_health(
    task_id: Option<i64>,
    checked_at: Option<chrono::DateTime<chrono::Utc>>,
    summary: Option<String>,
) -> AdminOverviewSystemHealthSummary {
    AdminOverviewSystemHealthSummary {
        status: AdminOverviewSystemHealthStatus::Unknown,
        components: Vec::new(),
        checked_at,
        task_id,
        summary,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use aster_forge_runtime::health::{HealthComponentDetail, HealthComponentDetailValue};
    use chrono::{Duration, Utc};
    use sea_orm::{ActiveModelTrait, Set};

    use super::{
        AdminOverviewServiceStatusKind, AdminOverviewSystemHealthStatus, RECENT_ACTIVITY_LIMIT,
        overview,
    };
    use crate::db::repository::{
        auth_session_repo, background_task_repo, minecraft_profile_repo, minecraft_texture_repo,
        user_repo, yggdrasil_token_repo,
    };
    use crate::runtime::{AppState, AppStateParts};
    use crate::services::ops::task::types::{
        RuntimeSystemHealthComponent, RuntimeSystemHealthResult, RuntimeSystemHealthStatus,
    };
    use crate::services::ops::task::{
        RuntimeTaskRunOutcome, SystemRuntimeTaskKind, record_runtime_task_run,
    };
    use aster_yggdrasil_model::entities::{
        auth_session, background_task, minecraft_profile, minecraft_texture, user,
    };
    use aster_yggdrasil_model::types::{
        audit::AuditAction, audit::AuditEntityType, task::BackgroundTaskKind,
        task::BackgroundTaskStatus, task::StoredTaskPayload, task::StoredTaskResult,
        user::UserRole, user::UserStatus, yggdrasil::MinecraftTextureLibraryStatus,
        yggdrasil::MinecraftTextureModel, yggdrasil::MinecraftTextureType,
        yggdrasil::MinecraftTextureVisibility,
    };
    async fn test_state() -> AppState {
        let texture_root = std::env::temp_dir().join(format!(
            "asteryggdrasil-admin-overview-{}",
            uuid::Uuid::new_v4()
        ));
        let db_cfg = crate::config::DatabaseConfig {
            url: "sqlite::memory:".into(),
            pool_size: 1,
            retry_count: 0,
        };
        let db = crate::db::connect_with_metrics(&db_cfg, aster_forge_metrics::NoopMetrics::arc())
            .await
            .expect("admin overview test database should connect");
        aster_yggdrasil_migration::Migrator::up(&db, None)
            .await
            .expect("admin overview test migrations should run");
        crate::services::ops::config::ensure_defaults(&db)
            .await
            .expect("admin overview test defaults should seed");

        let runtime_config = Arc::new(crate::config::RuntimeConfig::new());
        runtime_config
            .reload(&db)
            .await
            .expect("admin overview test runtime config should reload");
        let config = Arc::new(crate::config::Config {
            database: db_cfg,
            texture_storage: crate::config::TextureStorageConfig {
                backend: "local".to_string(),
                local_root: texture_root.to_string_lossy().to_string(),
                ..Default::default()
            },
            cache: aster_forge_cache::CacheConfig {
                ..Default::default()
            },
            ..Default::default()
        });
        let cache = aster_forge_cache::create_cache(&config.cache).await;
        let texture_storage = crate::texture_storage::create_texture_store(&config.texture_storage)
            .expect("admin overview test object storage should initialize");

        AppState::from_parts(AppStateParts {
            audit_writer: std::sync::Arc::new(aster_forge_audit::AuditWriter::new(db.clone())),
            db_handles: aster_forge_db::DbHandles::single(db),
            config,
            runtime_config,
            cache,
            texture_storage,
            mail_sender: aster_forge_mail::memory_sender(),
            config_sync: aster_forge_config::ConfigSyncRuntime::disabled_for_test(
                "aster_yggdrasil",
            ),
            metrics: aster_forge_metrics::NoopMetrics::arc(),
        })
        .expect("admin overview test AppState should build")
    }

    async fn insert_user(
        state: &AppState,
        username: &str,
    ) -> aster_yggdrasil_model::entities::user::Model {
        user_repo::create(
            state.writer_db(),
            username,
            &format!("{username}@example.com"),
            "password-hash",
            UserRole::User,
        )
        .await
        .expect("admin overview test user should insert")
    }

    async fn insert_user_at(
        state: &AppState,
        username: &str,
        created_at: chrono::DateTime<Utc>,
    ) -> aster_yggdrasil_model::entities::user::Model {
        user::ActiveModel {
            public_uuid: Set(uuid::Uuid::new_v4().to_string()),
            username: Set(username.to_string()),
            email: Set(format!("{username}@example.com")),
            password_hash: Set("password-hash".to_string()),
            role: Set(UserRole::User),
            status: Set(UserStatus::Active),
            must_change_password: Set(false),
            session_version: Set(0),
            email_verified_at: Set(None),
            pending_email: Set(None),
            created_at: Set(created_at),
            updated_at: Set(created_at),
            ..Default::default()
        }
        .insert(state.writer_db())
        .await
        .expect("admin overview test dated user should insert")
    }

    async fn insert_profile(
        state: &AppState,
        user_id: i64,
        name: &str,
    ) -> aster_yggdrasil_model::entities::minecraft_profile::Model {
        minecraft_profile_repo::create(
            state.writer_db(),
            user_id,
            &aster_forge_utils::id::new_short_token(),
            name,
            MinecraftTextureModel::Default,
            "skin,cape",
        )
        .await
        .expect("admin overview test profile should insert")
    }

    async fn insert_profile_at(
        state: &AppState,
        user_id: i64,
        name: &str,
        created_at: chrono::DateTime<Utc>,
    ) -> aster_yggdrasil_model::entities::minecraft_profile::Model {
        minecraft_profile::ActiveModel {
            user_id: Set(user_id),
            uuid: Set(aster_forge_utils::id::new_short_token()),
            name: Set(name.to_string()),
            texture_model: Set(MinecraftTextureModel::Default),
            uploadable_textures: Set("skin,cape".to_string()),
            created_at: Set(created_at),
            updated_at: Set(created_at),
            ..Default::default()
        }
        .insert(state.writer_db())
        .await
        .expect("admin overview test dated profile should insert")
    }

    async fn insert_texture(state: &AppState, user_id: i64, hash: &str) {
        minecraft_texture_repo::create(
            state.writer_db(),
            minecraft_texture_repo::CreateMinecraftTexture {
                user_id,
                texture_type: MinecraftTextureType::Skin,
                hash,
                storage_key: hash,
                mime_type: "image/png",
                file_size: 1,
                width: 64,
                height: 64,
                texture_model: MinecraftTextureModel::Default,
                visibility: MinecraftTextureVisibility::Private,
                is_wardrobe_item: true,
                display_name: None,
            },
        )
        .await
        .expect("admin overview test texture should insert");
    }

    async fn insert_texture_at(
        state: &AppState,
        user_id: i64,
        hash: &str,
        created_at: chrono::DateTime<Utc>,
    ) {
        minecraft_texture::ActiveModel {
            user_id: Set(user_id),
            texture_type: Set(MinecraftTextureType::Skin),
            hash: Set(hash.to_string()),
            storage_key: Set(hash.to_string()),
            mime_type: Set("image/png".to_string()),
            file_size: Set(1),
            width: Set(64),
            height: Set(64),
            texture_model: Set(MinecraftTextureModel::Default),
            visibility: Set(MinecraftTextureVisibility::Private),
            is_wardrobe_item: Set(true),
            display_name: Set(None),
            library_status: Set(MinecraftTextureLibraryStatus::Private),
            created_at: Set(created_at),
            updated_at: Set(created_at),
            ..Default::default()
        }
        .insert(state.writer_db())
        .await
        .expect("admin overview test dated texture should insert");
    }

    async fn insert_session(
        state: &AppState,
        user_id: i64,
        id: &str,
        refresh_expires_at: chrono::DateTime<Utc>,
        revoked_at: Option<chrono::DateTime<Utc>>,
    ) {
        let now = Utc::now();
        auth_session_repo::create(
            state.writer_db(),
            auth_session::ActiveModel {
                id: Set(id.to_string()),
                user_id: Set(user_id),
                current_refresh_jti: Set(format!("{id}-refresh")),
                previous_refresh_jti: Set(None),
                refresh_expires_at: Set(refresh_expires_at),
                user_agent: Set(None),
                ip_address: Set(None),
                created_at: Set(now),
                last_seen_at: Set(now),
                revoked_at: Set(revoked_at),
            },
        )
        .await
        .expect("admin overview test session should insert");
    }

    async fn insert_yggdrasil_token(
        state: &AppState,
        user_id: i64,
        access_hash: &str,
        selected_profile_id: Option<i64>,
        expires_at: chrono::DateTime<Utc>,
    ) {
        let now = Utc::now();
        yggdrasil_token_repo::create(
            state.writer_db(),
            yggdrasil_token_repo::CreateYggdrasilToken {
                user_id,
                access_token_hash: access_hash,
                client_token: access_hash,
                selected_profile_id,
                issued_at: now,
                expires_at,
                user_agent: None,
                ip_address: None,
            },
        )
        .await
        .expect("admin overview test yggdrasil token should insert");
    }

    async fn insert_task(state: &AppState, status: BackgroundTaskStatus, display_name: &str) {
        let now = Utc::now();
        background_task_repo::create(
            state.writer_db(),
            background_task::ActiveModel {
                kind: Set(BackgroundTaskKind::SystemRuntime),
                status: Set(status),
                creator_user_id: Set(None),
                display_name: Set(display_name.to_string()),
                payload_json: Set(StoredTaskPayload(
                    serde_json::json!({ "task_name": display_name }).to_string(),
                )),
                result_json: Set(None),
                runtime_json: Set(None),
                steps_json: Set(None),
                progress_current: Set(if status == BackgroundTaskStatus::Succeeded {
                    1
                } else {
                    0
                }),
                progress_total: Set(1),
                status_text: Set(None),
                attempt_count: Set(0),
                max_attempts: Set(3),
                next_run_at: Set(now),
                processing_token: Set(0),
                processing_started_at: Set(
                    (status == BackgroundTaskStatus::Processing).then_some(now)
                ),
                last_heartbeat_at: Set(None),
                lease_expires_at: Set(None),
                started_at: Set((status == BackgroundTaskStatus::Processing).then_some(now)),
                finished_at: Set(status.is_terminal().then_some(now)),
                last_error: Set(None),
                failure_can_retry: Set(None),
                expires_at: Set(now + Duration::hours(24)),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            },
        )
        .await
        .expect("admin overview test task should insert");
    }

    async fn insert_system_health_task(
        state: &AppState,
        status: RuntimeSystemHealthStatus,
        components: Vec<RuntimeSystemHealthComponent>,
    ) -> aster_yggdrasil_model::entities::background_task::Model {
        let finished_at = Utc::now();
        let started_at = finished_at - Duration::milliseconds(250);
        record_runtime_task_run(
            state,
            SystemRuntimeTaskKind::SystemHealthCheck,
            started_at,
            finished_at,
            &RuntimeTaskRunOutcome::succeeded_with_system_health(
                Some(format!("system health is {status:?}")),
                RuntimeSystemHealthResult { status, components },
            ),
        )
        .await
        .expect("admin overview test health task should record")
        .expect("admin overview test health task should be persisted")
    }

    async fn insert_yggdrasil_storage_consistency_task(
        state: &AppState,
        outcome: RuntimeTaskRunOutcome,
    ) -> aster_yggdrasil_model::entities::background_task::Model {
        let finished_at = Utc::now();
        let started_at = finished_at - Duration::milliseconds(150);
        record_runtime_task_run(
            state,
            SystemRuntimeTaskKind::YggdrasilStorageConsistencyCheck,
            started_at,
            finished_at,
            &outcome,
        )
        .await
        .expect("admin overview test storage consistency task should record")
        .expect("admin overview test storage consistency task should persist")
    }

    async fn insert_invalid_system_health_task(state: &AppState) {
        let now = Utc::now();
        background_task_repo::create(
            state.writer_db(),
            background_task::ActiveModel {
                kind: Set(BackgroundTaskKind::SystemRuntime),
                status: Set(BackgroundTaskStatus::Succeeded),
                creator_user_id: Set(None),
                display_name: Set(SystemRuntimeTaskKind::SystemHealthCheck
                    .display_name()
                    .to_string()),
                payload_json: Set(
                    crate::services::ops::task::runtime::system_runtime_payload_json(
                        SystemRuntimeTaskKind::SystemHealthCheck,
                    )
                    .expect("admin overview test health payload should serialize"),
                ),
                result_json: Set(Some(StoredTaskResult("{not-json".to_string()))),
                runtime_json: Set(None),
                steps_json: Set(None),
                progress_current: Set(1),
                progress_total: Set(1),
                status_text: Set(Some("latest result is unreadable".to_string())),
                attempt_count: Set(0),
                max_attempts: Set(3),
                next_run_at: Set(now),
                processing_token: Set(0),
                processing_started_at: Set(None),
                last_heartbeat_at: Set(None),
                lease_expires_at: Set(None),
                started_at: Set(Some(now - Duration::milliseconds(100))),
                finished_at: Set(Some(now)),
                last_error: Set(None),
                failure_can_retry: Set(None),
                expires_at: Set(now + Duration::hours(24)),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            },
        )
        .await
        .expect("admin overview test invalid health task should insert");
    }

    async fn insert_audit_log(state: &AppState, id: i64, created_at: chrono::DateTime<Utc>) {
        insert_audit_log_with_action(state, id, AuditAction::UserLogin, created_at).await;
    }

    async fn insert_audit_log_with_action(
        state: &AppState,
        id: i64,
        action: AuditAction,
        created_at: chrono::DateTime<Utc>,
    ) {
        aster_forge_db::create_audit_log_row(
            state.writer_db(),
            aster_forge_db::AuditLogCreate {
                user_id: 1,
                action: action.as_str().to_string(),
                entity_type: audit_entity_type_for_action(action).as_str().to_string(),
                entity_id: Some(id),
                entity_name: Some(format!("user-{id}")),
                details: Some(serde_json::json!({ "id": id }).to_string()),
                ip_address: Some("127.0.0.1".to_string()),
                user_agent: Some("admin-overview-test".to_string()),
                created_at,
            },
        )
        .await
        .expect("admin overview test audit log should insert");
    }

    async fn insert_yggdrasil_audit_log(
        state: &AppState,
        id: i64,
        action: AuditAction,
        user_id: i64,
        profile: Option<&aster_yggdrasil_model::entities::minecraft_profile::Model>,
        created_at: chrono::DateTime<Utc>,
    ) {
        let entity_id = if action == AuditAction::YggdrasilJoinServer {
            profile.map(|profile| profile.id)
        } else {
            Some(id)
        };
        let details = profile.map(|profile| {
            if action == AuditAction::YggdrasilAuthenticate {
                serde_json::json!({
                    "identifier": format!("user-{user_id}"),
                    "selected_profile_uuid": profile.uuid.as_str(),
                    "selected_profile_name": profile.name.as_str(),
                    "available_profile_count": 1,
                })
            } else {
                serde_json::json!({
                    "profile_uuid": profile.uuid.as_str(),
                    "profile_name": profile.name.as_str(),
                })
            }
            .to_string()
        });

        aster_forge_db::create_audit_log_row(
            state.writer_db(),
            aster_forge_db::AuditLogCreate {
                user_id,
                action: action.as_str().to_string(),
                entity_type: audit_entity_type_for_action(action).as_str().to_string(),
                entity_id,
                entity_name: profile.map(|profile| profile.name.clone()),
                details,
                ip_address: Some("127.0.0.1".to_string()),
                user_agent: Some("admin-overview-test".to_string()),
                created_at,
            },
        )
        .await
        .expect("admin overview test yggdrasil audit log should insert");
    }

    fn audit_entity_type_for_action(action: AuditAction) -> AuditEntityType {
        match action {
            AuditAction::YggdrasilJoinServer => AuditEntityType::YggdrasilSession,
            AuditAction::YggdrasilAuthenticate
            | AuditAction::YggdrasilRefreshToken
            | AuditAction::YggdrasilInvalidateToken
            | AuditAction::YggdrasilSignout => AuditEntityType::YggdrasilToken,
            _ => AuditEntityType::User,
        }
    }

    #[tokio::test]
    async fn overview_counts_existing_domain_records() {
        let state = test_state().await;
        let user = insert_user(&state, "overview-user").await;
        let profile = insert_profile(&state, user.id, "OverviewPlayer").await;
        insert_texture(&state, user.id, "overview-texture.png").await;
        let now = Utc::now();
        insert_session(
            &state,
            user.id,
            "overview-session",
            now + Duration::hours(1),
            None,
        )
        .await;
        insert_yggdrasil_token(
            &state,
            user.id,
            "overview-token",
            Some(profile.id),
            now + Duration::hours(1),
        )
        .await;

        let response = overview(&state).await.unwrap();

        assert_eq!(response.summary.total_users, 1);
        assert_eq!(response.summary.minecraft_profile_count, 1);
        assert_eq!(response.summary.texture_count, 1);
        assert_eq!(response.summary.active_session_count, 1);
        assert_eq!(response.summary.active_yggdrasil_token_count, 1);
        assert!(
            response
                .services
                .iter()
                .any(|service| service.key == "database")
        );
    }

    #[tokio::test]
    async fn overview_returns_zero_counts_for_empty_instance() {
        let state = test_state().await;

        let response = overview(&state).await.unwrap();

        assert_eq!(response.summary.total_users, 0);
        assert_eq!(response.summary.minecraft_profile_count, 0);
        assert_eq!(response.summary.texture_count, 0);
        assert_eq!(response.summary.active_session_count, 0);
        assert_eq!(response.summary.active_yggdrasil_token_count, 0);
        assert_eq!(response.summary.processing_task_count, 0);
        assert_eq!(response.summary.pending_task_count, 0);
        assert!(response.recent_activity.is_empty());
        assert_eq!(response.activity_trend.len(), 7);
        assert!(
            response
                .activity_trend
                .iter()
                .all(|point| point.active_users == 0
                    && point.active_players == 0
                    && point.new_textures == 0
                    && point.yggdrasil_api_calls == 0)
        );
        assert!(
            response
                .services
                .iter()
                .all(|service| service.status == AdminOverviewServiceStatusKind::Ok)
        );
        assert_eq!(
            response.system_health.status,
            AdminOverviewSystemHealthStatus::Unknown
        );
        assert!(response.system_health.components.is_empty());
        assert_eq!(response.system_health.task_id, None);
        assert_eq!(response.system_health.checked_at, None);
    }

    #[tokio::test]
    async fn overview_returns_seven_day_activity_trend_by_utc_day() {
        let state = test_state().await;
        let today = Utc::now().date_naive();
        let today_start = today.and_hms_opt(0, 0, 0).unwrap().and_utc();
        let two_days_ago = today_start - Duration::days(2);
        let outside_window = today_start - Duration::days(8);

        let old_user = insert_user_at(&state, "trend-old-user", two_days_ago).await;
        let today_user = insert_user_at(&state, "trend-today-user", today_start).await;
        let _outside_user = insert_user_at(&state, "trend-outside-user", outside_window).await;
        let old_profile =
            insert_profile_at(&state, old_user.id, "TrendOldProfile", two_days_ago).await;
        let today_profile =
            insert_profile_at(&state, today_user.id, "TrendTodayProfile", today_start).await;
        insert_texture_at(&state, old_user.id, "trend-old-texture.png", two_days_ago).await;
        insert_texture_at(
            &state,
            today_user.id,
            "trend-today-texture.png",
            today_start,
        )
        .await;
        insert_yggdrasil_audit_log(
            &state,
            900,
            AuditAction::YggdrasilAuthenticate,
            old_user.id,
            Some(&old_profile),
            two_days_ago,
        )
        .await;
        insert_yggdrasil_audit_log(
            &state,
            901,
            AuditAction::YggdrasilJoinServer,
            today_user.id,
            Some(&today_profile),
            today_start,
        )
        .await;
        insert_yggdrasil_audit_log(
            &state,
            904,
            AuditAction::YggdrasilRefreshToken,
            today_user.id,
            Some(&today_profile),
            today_start + Duration::minutes(10),
        )
        .await;
        insert_yggdrasil_audit_log(
            &state,
            902,
            AuditAction::YggdrasilRefreshToken,
            old_user.id,
            Some(&old_profile),
            outside_window,
        )
        .await;
        insert_audit_log(&state, 903, today_start).await;

        let response = overview(&state).await.unwrap();
        let dates = response
            .activity_trend
            .iter()
            .map(|point| point.date.as_str())
            .collect::<Vec<_>>();
        let two_days_ago_key = (today - Duration::days(2)).format("%Y-%m-%d").to_string();
        let today_key = today.format("%Y-%m-%d").to_string();
        let old_point = response
            .activity_trend
            .iter()
            .find(|point| point.date == two_days_ago_key)
            .expect("two days ago trend point should be present");
        let today_point = response
            .activity_trend
            .iter()
            .find(|point| point.date == today_key)
            .expect("today trend point should be present");

        assert_eq!(response.activity_trend.len(), 7);
        assert!(dates.windows(2).all(|window| window[0] < window[1]));
        assert_eq!(old_point.active_users, 1);
        assert_eq!(old_point.active_players, 1);
        assert_eq!(old_point.new_textures, 1);
        assert_eq!(old_point.yggdrasil_api_calls, 1);
        assert_eq!(today_point.active_users, 2);
        assert_eq!(today_point.active_players, 1);
        assert_eq!(today_point.new_textures, 1);
        assert_eq!(today_point.yggdrasil_api_calls, 2);
        assert_eq!(
            response
                .activity_trend
                .iter()
                .map(|point| point.active_users)
                .sum::<u64>(),
            3
        );
        assert_eq!(
            response
                .activity_trend
                .iter()
                .map(|point| point.yggdrasil_api_calls)
                .sum::<u64>(),
            3
        );
    }

    #[tokio::test]
    async fn overview_reports_latest_healthy_system_health_check() {
        let state = test_state().await;
        let task = insert_system_health_task(
            &state,
            RuntimeSystemHealthStatus::Healthy,
            vec![RuntimeSystemHealthComponent {
                name: "database".to_string(),
                status: RuntimeSystemHealthStatus::Healthy,
                message: "database check passed".to_string(),
                details: Vec::new(),
            }],
        )
        .await;

        let response = overview(&state).await.unwrap();

        assert_eq!(
            response.system_health.status,
            AdminOverviewSystemHealthStatus::Healthy
        );
        assert_eq!(response.system_health.task_id, Some(task.id));
        assert!(response.system_health.checked_at.is_some());
        assert_eq!(response.system_health.components.len(), 1);
        assert_eq!(response.system_health.components[0].name, "database");
        assert_eq!(
            response.system_health.components[0].status,
            AdminOverviewSystemHealthStatus::Healthy
        );
    }

    #[tokio::test]
    async fn overview_merges_healthy_yggdrasil_storage_consistency_observation() {
        let state = test_state().await;
        insert_system_health_task(
            &state,
            RuntimeSystemHealthStatus::Healthy,
            vec![RuntimeSystemHealthComponent {
                name: "database".to_string(),
                status: RuntimeSystemHealthStatus::Healthy,
                message: "database check passed".to_string(),
                details: Vec::new(),
            }],
        )
        .await;
        insert_yggdrasil_storage_consistency_task(
            &state,
            RuntimeTaskRunOutcome::succeeded(Some("checked 3 object storage records".to_string())),
        )
        .await;

        let response = overview(&state).await.unwrap();
        let storage = response
            .system_health
            .components
            .iter()
            .find(|component| component.name == "yggdrasil_storage_consistency")
            .expect("storage consistency observation should be included");

        assert_eq!(
            response.system_health.status,
            AdminOverviewSystemHealthStatus::Healthy
        );
        assert!(response.system_health.task_id.is_some());
        assert_eq!(storage.status, AdminOverviewSystemHealthStatus::Healthy);
        assert_eq!(storage.message, "checked 3 object storage records");
    }

    #[tokio::test]
    async fn overview_merges_failed_yggdrasil_storage_consistency_observation() {
        let state = test_state().await;
        insert_system_health_task(
            &state,
            RuntimeSystemHealthStatus::Healthy,
            vec![RuntimeSystemHealthComponent {
                name: "database".to_string(),
                status: RuntimeSystemHealthStatus::Healthy,
                message: "database check passed".to_string(),
                details: Vec::new(),
            }],
        )
        .await;
        insert_yggdrasil_storage_consistency_task(
            &state,
            RuntimeTaskRunOutcome::failed(
                Some("checked 5, missing 1, hash/key mismatched 1 texture blobs".to_string()),
                "checked 5, missing 1, hash/key mismatched 1 texture blobs",
            ),
        )
        .await;

        let response = overview(&state).await.unwrap();
        let storage = response
            .system_health
            .components
            .iter()
            .find(|component| component.name == "yggdrasil_storage_consistency")
            .expect("storage consistency observation should be included");

        assert_eq!(
            response.system_health.status,
            AdminOverviewSystemHealthStatus::Unhealthy
        );
        assert_eq!(storage.status, AdminOverviewSystemHealthStatus::Unhealthy);
        assert_eq!(
            storage.message,
            "checked 5, missing 1, hash/key mismatched 1 texture blobs"
        );
    }

    #[tokio::test]
    async fn overview_reports_degraded_and_unhealthy_components() {
        let state = test_state().await;
        insert_system_health_task(
            &state,
            RuntimeSystemHealthStatus::Degraded,
            vec![
                RuntimeSystemHealthComponent {
                    name: "database".to_string(),
                    status: RuntimeSystemHealthStatus::Healthy,
                    message: "database check passed".to_string(),
                    details: Vec::new(),
                },
                RuntimeSystemHealthComponent {
                    name: "cache".to_string(),
                    status: RuntimeSystemHealthStatus::Degraded,
                    message: "cache backend is unavailable; using fallback".to_string(),
                    details: vec![HealthComponentDetail::new("active_backend", "memory")],
                },
            ],
        )
        .await;

        let response = overview(&state).await.unwrap();

        assert_eq!(
            response.system_health.status,
            AdminOverviewSystemHealthStatus::Degraded
        );
        let cache = response
            .system_health
            .components
            .iter()
            .find(|component| component.name == "cache")
            .expect("cache component should be included");
        assert_eq!(cache.status, AdminOverviewSystemHealthStatus::Degraded);
        assert_eq!(
            cache.message,
            "cache backend is unavailable; using fallback"
        );
        assert_eq!(cache.details.len(), 1);
        assert_eq!(cache.details[0].key, "active_backend");
        assert_eq!(
            cache.details[0].value,
            HealthComponentDetailValue::Text("memory".to_string())
        );
    }

    #[tokio::test]
    async fn overview_reports_unhealthy_system_health_check() {
        let state = test_state().await;
        insert_system_health_task(
            &state,
            RuntimeSystemHealthStatus::Unhealthy,
            vec![RuntimeSystemHealthComponent {
                name: "background_tasks".to_string(),
                status: RuntimeSystemHealthStatus::Unhealthy,
                message: "dispatcher has not reported recently".to_string(),
                details: Vec::new(),
            }],
        )
        .await;

        let response = overview(&state).await.unwrap();

        assert_eq!(
            response.system_health.status,
            AdminOverviewSystemHealthStatus::Unhealthy
        );
        assert_eq!(
            response.system_health.components[0].status,
            AdminOverviewSystemHealthStatus::Unhealthy
        );
    }

    #[tokio::test]
    async fn overview_keeps_loading_when_system_health_result_is_invalid() {
        let state = test_state().await;
        insert_invalid_system_health_task(&state).await;

        let response = overview(&state).await.unwrap();

        assert_eq!(
            response.system_health.status,
            AdminOverviewSystemHealthStatus::Unknown
        );
        assert!(response.system_health.task_id.is_some());
        assert!(response.system_health.checked_at.is_some());
        assert_eq!(
            response.system_health.summary.as_deref(),
            Some("latest result is unreadable")
        );
        assert!(response.system_health.components.is_empty());
    }

    #[tokio::test]
    async fn overview_excludes_expired_revoked_and_temporarily_invalidated_auth_records() {
        let state = test_state().await;
        let user = insert_user(&state, "auth-boundary-user").await;
        let profile = insert_profile(&state, user.id, "BoundaryPlayer").await;
        let now = Utc::now();

        insert_session(
            &state,
            user.id,
            "active-session",
            now + Duration::hours(1),
            None,
        )
        .await;
        insert_session(
            &state,
            user.id,
            "expired-session",
            now - Duration::seconds(1),
            None,
        )
        .await;
        insert_session(
            &state,
            user.id,
            "revoked-session",
            now + Duration::hours(1),
            Some(now),
        )
        .await;

        insert_yggdrasil_token(
            &state,
            user.id,
            "active-token",
            Some(profile.id),
            now + Duration::hours(1),
        )
        .await;
        insert_yggdrasil_token(
            &state,
            user.id,
            "expired-token",
            Some(profile.id),
            now - Duration::seconds(1),
        )
        .await;
        insert_yggdrasil_token(
            &state,
            user.id,
            "revoked-token",
            Some(profile.id),
            now + Duration::hours(1),
        )
        .await;
        yggdrasil_token_repo::revoke_by_access_hash(state.writer_db(), "revoked-token")
            .await
            .unwrap();
        insert_yggdrasil_token(
            &state,
            user.id,
            "temporarily-invalid-token",
            Some(profile.id),
            now + Duration::hours(1),
        )
        .await;
        yggdrasil_token_repo::temporarily_invalidate_all_for_selected_profile(
            state.writer_db(),
            profile.id,
        )
        .await
        .unwrap();
        insert_yggdrasil_token(
            &state,
            user.id,
            "active-token-after-invalidation",
            None,
            now + Duration::hours(1),
        )
        .await;

        let response = overview(&state).await.unwrap();

        assert_eq!(response.summary.active_session_count, 1);
        assert_eq!(response.summary.active_yggdrasil_token_count, 1);
    }

    #[tokio::test]
    async fn overview_reports_task_counts_and_warning_only_for_pending_or_retry_tasks() {
        let state = test_state().await;
        insert_task(&state, BackgroundTaskStatus::Processing, "processing").await;
        insert_task(&state, BackgroundTaskStatus::Pending, "pending").await;
        insert_task(&state, BackgroundTaskStatus::Retry, "retry").await;
        insert_task(&state, BackgroundTaskStatus::Succeeded, "succeeded").await;
        insert_task(&state, BackgroundTaskStatus::Failed, "failed").await;

        let response = overview(&state).await.unwrap();
        let task_service = response
            .services
            .iter()
            .find(|service| service.key == "background_tasks")
            .expect("background task status should be present");

        assert_eq!(response.summary.processing_task_count, 1);
        assert_eq!(response.summary.pending_task_count, 2);
        assert_eq!(task_service.status, AdminOverviewServiceStatusKind::Warning);
        assert_eq!(
            task_service.metric.as_deref(),
            Some("1 processing / 2 queued")
        );
    }

    #[tokio::test]
    async fn overview_limits_recent_activity_to_latest_entries() {
        let state = test_state().await;
        let base = Utc::now() - Duration::minutes(30);
        for index in 0..(RECENT_ACTIVITY_LIMIT + 3) {
            insert_audit_log(&state, index as i64, base + Duration::seconds(index as i64)).await;
        }

        let response = overview(&state).await.unwrap();

        assert_eq!(
            response.recent_activity.len(),
            aster_forge_utils::numbers::u64_to_usize(
                RECENT_ACTIVITY_LIMIT,
                "recent activity test limit"
            )
            .unwrap()
        );
        assert_eq!(response.recent_activity[0].entity_id, Some(8));
        assert_eq!(response.recent_activity[5].entity_id, Some(3));
    }
}
