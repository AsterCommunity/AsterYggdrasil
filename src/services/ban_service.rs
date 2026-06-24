//! User capability ban service.

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::api::error_code::AsterErrorCode;
use crate::db::repository::{user_ban_repo, user_repo};
use crate::entities::{user_ban, user_ban_event};
use crate::errors::{AsterError, Result};
use crate::runtime::DatabaseRuntimeState;
use crate::types::{
    UserBanEventType, UserBanScope, UserBanScopes, UserBanScopesError, UserBanStatus,
};
use aster_forge_api::{CursorPage, DateTimeIdCursor, NullablePatch};

const REASON_MAX_CHARS: usize = 128;
const NOTE_MAX_CHARS: usize = 1000;

#[derive(Debug, Clone)]
pub struct CreateUserBanInput {
    pub target_user_id: i64,
    pub scopes: Vec<UserBanScope>,
    pub reason: String,
    pub public_reason: Option<String>,
    pub admin_note: Option<String>,
    pub starts_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateUserBanInput {
    pub scopes: Option<Vec<UserBanScope>>,
    pub reason: Option<String>,
    pub public_reason: Option<NullablePatch<String>>,
    pub admin_note: Option<NullablePatch<String>>,
    pub starts_at: Option<DateTime<Utc>>,
    pub expires_at: Option<NullablePatch<DateTime<Utc>>>,
}

#[derive(Debug, Clone, Default)]
pub struct ListUserBansInput {
    pub limit: u64,
    pub cursor: Option<(DateTime<Utc>, i64)>,
    pub user_id: Option<i64>,
    pub scope: Option<UserBanScope>,
    pub status: Option<UserBanStatus>,
    pub effective_only: bool,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
pub struct UserBanInfo {
    pub id: i64,
    pub user_id: i64,
    pub scopes: Vec<UserBanScope>,
    pub status: UserBanStatus,
    pub effective_status: UserBanStatus,
    pub effective: bool,
    pub reason: String,
    pub public_reason: Option<String>,
    pub admin_note: Option<String>,
    pub created_by_user_id: Option<i64>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub starts_at: DateTime<Utc>,
    #[cfg_attr(
        all(debug_assertions, feature = "openapi"),
        schema(value_type = Option<String>)
    )]
    pub expires_at: Option<DateTime<Utc>>,
    #[cfg_attr(
        all(debug_assertions, feature = "openapi"),
        schema(value_type = Option<String>)
    )]
    pub revoked_at: Option<DateTime<Utc>>,
    pub revoked_by_user_id: Option<i64>,
    pub revoke_note: Option<String>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: DateTime<Utc>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(utoipa::ToSchema))]
pub struct UserBanEventInfo {
    pub id: i64,
    pub ban_id: i64,
    pub actor_user_id: Option<i64>,
    pub event_type: UserBanEventType,
    pub previous_status: Option<UserBanStatus>,
    pub next_status: Option<UserBanStatus>,
    pub previous_scopes: Option<Vec<UserBanScope>>,
    pub next_scopes: Option<Vec<UserBanScope>>,
    #[cfg_attr(
        all(debug_assertions, feature = "openapi"),
        schema(value_type = Option<String>)
    )]
    pub previous_expires_at: Option<DateTime<Utc>>,
    #[cfg_attr(
        all(debug_assertions, feature = "openapi"),
        schema(value_type = Option<String>)
    )]
    pub next_expires_at: Option<DateTime<Utc>>,
    pub note: Option<String>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    pub created_at: DateTime<Utc>,
}

pub async fn ensure_user_not_banned<S>(state: &S, user_id: i64, scope: UserBanScope) -> Result<()>
where
    S: DatabaseRuntimeState,
{
    if let Some(ban) = active_user_ban_for_scope(state, user_id, scope).await? {
        tracing::debug!(
            user_id,
            ban_id = ban.id,
            scope = scope.as_str(),
            "user capability rejected by active ban"
        );
        return Err(AsterError::auth_forbidden_code(
            AsterErrorCode::UserBanForbidden,
            format!("user is banned from {}", scope.as_str()),
        ));
    }
    Ok(())
}

pub async fn is_user_banned<S>(state: &S, user_id: i64, scope: UserBanScope) -> Result<bool>
where
    S: DatabaseRuntimeState,
{
    Ok(active_user_ban_for_scope(state, user_id, scope)
        .await?
        .is_some())
}

pub async fn active_user_ban_for_scope<S>(
    state: &S,
    user_id: i64,
    scope: UserBanScope,
) -> Result<Option<user_ban::Model>>
where
    S: DatabaseRuntimeState,
{
    user_ban_repo::find_effective_for_scope(state.reader_db(), user_id, scope, Utc::now()).await
}

pub async fn active_user_bans<S>(state: &S, user_id: i64) -> Result<Vec<UserBanInfo>>
where
    S: DatabaseRuntimeState,
{
    let now = Utc::now();
    let bans = user_ban_repo::list_effective_for_user(state.reader_db(), user_id, now).await?;
    Ok(bans.iter().map(|ban| ban_info_at(ban, now)).collect())
}

pub async fn list_user_bans<S>(
    state: &S,
    input: ListUserBansInput,
) -> Result<CursorPage<UserBanInfo, DateTimeIdCursor>>
where
    S: DatabaseRuntimeState,
{
    let scope_filter = input.scope;
    let page = user_ban_repo::list_cursor(
        state.reader_db(),
        input.limit,
        user_ban_repo::UserBanListFilter {
            user_id: input.user_id,
            status: input.status,
            effective_only: input.effective_only,
        },
        input.cursor,
    )
    .await?;
    let next_cursor = if page.has_more {
        page.items.last().map(|ban| DateTimeIdCursor {
            value: ban.created_at,
            id: ban.id,
        })
    } else {
        None
    };
    let now = Utc::now();
    let items = page
        .items
        .iter()
        .map(|ban| ban_info_at(ban, now))
        .filter(|ban| scope_filter.is_none_or(|scope| ban.scopes.contains(&scope)))
        .collect::<Vec<_>>();
    let total = if scope_filter.is_some() {
        aster_forge_utils::numbers::usize_to_u64(items.len(), "filtered user ban total")?
    } else {
        page.total
    };
    Ok(CursorPage::new(
        items,
        total,
        input.limit.clamp(1, 100),
        next_cursor,
    ))
}

pub async fn get_user_ban<S>(state: &S, ban_id: i64) -> Result<UserBanInfo>
where
    S: DatabaseRuntimeState,
{
    let ban = user_ban(state, ban_id).await?;
    Ok(ban_info(&ban))
}

pub async fn create_user_ban<S>(
    state: &S,
    actor_user_id: i64,
    input: CreateUserBanInput,
) -> Result<UserBanInfo>
where
    S: DatabaseRuntimeState,
{
    user_repo::find_by_id(state.reader_db(), input.target_user_id).await?;
    let starts_at = input.starts_at.unwrap_or_else(Utc::now);
    validate_time_range(starts_at, input.expires_at)?;
    let reason = normalize_required_text(
        input.reason,
        REASON_MAX_CHARS,
        AsterErrorCode::UserBanReasonInvalid,
        "ban reason",
    )?;
    let public_reason = normalize_optional_text(
        input.public_reason,
        NOTE_MAX_CHARS,
        AsterErrorCode::UserBanReasonInvalid,
        "public ban reason",
    )?;
    let admin_note = normalize_optional_text(
        input.admin_note,
        NOTE_MAX_CHARS,
        AsterErrorCode::UserBanReasonInvalid,
        "ban admin note",
    )?;

    let scopes = normalize_scopes(input.scopes)?;
    reject_duplicate_effective_scopes(state, input.target_user_id, scopes.as_slice(), None).await?;

    let ban = crate::db::transaction::with_transaction(state.writer_db(), async |txn| {
        let ban = user_ban_repo::create(
            txn,
            user_ban_repo::CreateUserBan {
                user_id: input.target_user_id,
                scopes: UserBanScopes::new(scopes.clone()).map_err(scope_error)?,
                reason,
                public_reason,
                admin_note,
                created_by_user_id: Some(actor_user_id),
                starts_at,
                expires_at: input.expires_at,
            },
        )
        .await?;
        user_ban_repo::create_event(
            txn,
            user_ban_repo::CreateUserBanEvent {
                ban_id: ban.id,
                actor_user_id: Some(actor_user_id),
                event_type: UserBanEventType::Created,
                previous_status: None,
                next_status: Some(ban.status),
                previous_scopes: None,
                next_scopes: Some(ban.scopes.clone()),
                previous_expires_at: None,
                next_expires_at: ban.expires_at,
                note: Some(ban.reason.clone()),
            },
        )
        .await?;
        Ok(ban)
    })
    .await?;

    Ok(ban_info(&ban))
}

pub async fn update_user_ban<S>(
    state: &S,
    actor_user_id: i64,
    ban_id: i64,
    input: UpdateUserBanInput,
) -> Result<UserBanInfo>
where
    S: DatabaseRuntimeState,
{
    let ban = user_ban(state, ban_id).await?;
    if !is_effective(&ban, Utc::now()) {
        return Err(AsterError::validation_error_code(
            AsterErrorCode::UserBanNotActive,
            "only active user bans can be updated",
        ));
    }

    let previous_scopes = ban_scopes(&ban)?;
    let next_scopes = match input.scopes {
        Some(scopes) => normalize_scopes(scopes)?,
        None => previous_scopes.clone(),
    };
    let next_starts_at = input.starts_at.unwrap_or(ban.starts_at);
    let next_expires_at = match input.expires_at {
        Some(NullablePatch::Null) => None,
        Some(NullablePatch::Value(value)) => Some(value),
        Some(NullablePatch::Absent) | None => ban.expires_at,
    };
    validate_time_range(next_starts_at, next_expires_at)?;
    if next_scopes != previous_scopes {
        reject_duplicate_effective_scopes(state, ban.user_id, next_scopes.as_slice(), Some(ban.id))
            .await?;
    }

    let reason = input
        .reason
        .map(|value| {
            normalize_required_text(
                value,
                REASON_MAX_CHARS,
                AsterErrorCode::UserBanReasonInvalid,
                "ban reason",
            )
        })
        .transpose()?;
    let public_reason = normalize_nullable_text_patch(
        input.public_reason,
        NOTE_MAX_CHARS,
        AsterErrorCode::UserBanReasonInvalid,
        "public ban reason",
    )?;
    let admin_note = normalize_nullable_text_patch(
        input.admin_note,
        NOTE_MAX_CHARS,
        AsterErrorCode::UserBanReasonInvalid,
        "ban admin note",
    )?;

    let previous_status = ban.status;
    let previous_expires_at = ban.expires_at;
    let next_scopes_value = UserBanScopes::new(next_scopes.clone()).map_err(scope_error)?;
    let updated = crate::db::transaction::with_transaction(state.writer_db(), async |txn| {
        let updated = user_ban_repo::update(
            txn,
            ban,
            user_ban_repo::UpdateUserBan {
                scopes: (next_scopes != previous_scopes).then_some(next_scopes_value),
                reason,
                public_reason,
                admin_note,
                starts_at: input.starts_at,
                expires_at: Some(next_expires_at)
                    .filter(|_| next_expires_at != previous_expires_at),
            },
        )
        .await?;
        user_ban_repo::create_event(
            txn,
            user_ban_repo::CreateUserBanEvent {
                ban_id: updated.id,
                actor_user_id: Some(actor_user_id),
                event_type: UserBanEventType::Updated,
                previous_status: Some(previous_status),
                next_status: Some(updated.status),
                previous_scopes: Some(UserBanScopes::new(previous_scopes).map_err(scope_error)?),
                next_scopes: Some(updated.scopes.clone()),
                previous_expires_at,
                next_expires_at: updated.expires_at,
                note: updated.admin_note.clone(),
            },
        )
        .await?;
        Ok(updated)
    })
    .await?;
    Ok(ban_info(&updated))
}

pub async fn revoke_user_ban<S>(
    state: &S,
    actor_user_id: i64,
    ban_id: i64,
    revoke_note: Option<String>,
) -> Result<UserBanInfo>
where
    S: DatabaseRuntimeState,
{
    let ban = user_ban(state, ban_id).await?;
    if !is_effective(&ban, Utc::now()) {
        return Err(AsterError::validation_error_code(
            AsterErrorCode::UserBanNotActive,
            "only active user bans can be revoked",
        ));
    }
    let revoke_note = normalize_optional_text(
        revoke_note,
        NOTE_MAX_CHARS,
        AsterErrorCode::UserBanReasonInvalid,
        "ban revoke note",
    )?;
    let previous_status = ban.status;
    let previous_scopes = ban_scopes(&ban)?;
    let previous_expires_at = ban.expires_at;
    let revoked_at = Utc::now();
    let updated = crate::db::transaction::with_transaction(state.writer_db(), async |txn| {
        let updated = user_ban_repo::revoke(
            txn,
            ban,
            Some(actor_user_id),
            revoke_note.clone(),
            revoked_at,
        )
        .await?;
        user_ban_repo::create_event(
            txn,
            user_ban_repo::CreateUserBanEvent {
                ban_id: updated.id,
                actor_user_id: Some(actor_user_id),
                event_type: UserBanEventType::Revoked,
                previous_status: Some(previous_status),
                next_status: Some(updated.status),
                previous_scopes: Some(UserBanScopes::new(previous_scopes).map_err(scope_error)?),
                next_scopes: Some(updated.scopes.clone()),
                previous_expires_at,
                next_expires_at: updated.expires_at,
                note: revoke_note,
            },
        )
        .await?;
        Ok(updated)
    })
    .await?;
    Ok(ban_info(&updated))
}

pub async fn list_user_ban_events<S>(state: &S, ban_id: i64) -> Result<Vec<UserBanEventInfo>>
where
    S: DatabaseRuntimeState,
{
    user_ban(state, ban_id).await?;
    let events = user_ban_repo::list_events(state.reader_db(), ban_id).await?;
    Ok(events.iter().map(event_info).collect())
}

async fn user_ban<S>(state: &S, ban_id: i64) -> Result<user_ban::Model>
where
    S: DatabaseRuntimeState,
{
    if ban_id <= 0 {
        return Err(AsterError::record_not_found_code(
            AsterErrorCode::UserBanNotFound,
            "invalid user ban id",
        ));
    }
    user_ban_repo::find_by_id(state.reader_db(), ban_id)
        .await?
        .ok_or_else(|| {
            AsterError::record_not_found_code(
                AsterErrorCode::UserBanNotFound,
                format!("user ban '{ban_id}'"),
            )
        })
}

async fn reject_duplicate_effective_scopes<S>(
    state: &S,
    user_id: i64,
    scopes: &[UserBanScope],
    excluding_ban_id: Option<i64>,
) -> Result<()>
where
    S: DatabaseRuntimeState,
{
    for scope in scopes {
        if user_ban_repo::find_effective_for_scope(state.reader_db(), user_id, *scope, Utc::now())
            .await?
            .is_some_and(|ban| Some(ban.id) != excluding_ban_id)
        {
            return Err(AsterError::validation_error_code(
                AsterErrorCode::UserBanAlreadyActive,
                "an active user ban already exists for one of these scopes",
            ));
        }
    }
    Ok(())
}

fn ban_info(ban: &user_ban::Model) -> UserBanInfo {
    ban_info_at(ban, Utc::now())
}

fn ban_info_at(ban: &user_ban::Model, now: DateTime<Utc>) -> UserBanInfo {
    let effective = is_effective(ban, now);
    let effective_status = if ban.status == UserBanStatus::Active && !effective {
        UserBanStatus::Expired
    } else {
        ban.status
    };
    UserBanInfo {
        id: ban.id,
        user_id: ban.user_id,
        scopes: ban_scopes(ban).unwrap_or_else(|_| Vec::new()),
        status: ban.status,
        effective_status,
        effective,
        reason: ban.reason.clone(),
        public_reason: ban.public_reason.clone(),
        admin_note: ban.admin_note.clone(),
        created_by_user_id: ban.created_by_user_id,
        starts_at: ban.starts_at,
        expires_at: ban.expires_at,
        revoked_at: ban.revoked_at,
        revoked_by_user_id: ban.revoked_by_user_id,
        revoke_note: ban.revoke_note.clone(),
        created_at: ban.created_at,
        updated_at: ban.updated_at,
    }
}

fn event_info(event: &user_ban_event::Model) -> UserBanEventInfo {
    UserBanEventInfo {
        id: event.id,
        ban_id: event.ban_id,
        actor_user_id: event.actor_user_id,
        event_type: event.event_type,
        previous_status: event.previous_status,
        next_status: event.next_status,
        previous_scopes: event
            .previous_scopes
            .as_ref()
            .and_then(|scopes| scopes.as_vec().ok()),
        next_scopes: event
            .next_scopes
            .as_ref()
            .and_then(|scopes| scopes.as_vec().ok()),
        previous_expires_at: event.previous_expires_at,
        next_expires_at: event.next_expires_at,
        note: event.note.clone(),
        created_at: event.created_at,
    }
}

fn normalize_scopes(scopes: Vec<UserBanScope>) -> Result<Vec<UserBanScope>> {
    UserBanScopes::new(scopes)
        .and_then(|scopes| scopes.as_vec())
        .map_err(scope_error)
}

fn ban_scopes(ban: &user_ban::Model) -> Result<Vec<UserBanScope>> {
    ban.scopes.as_vec().map_err(scope_error)
}

fn scope_error(error: UserBanScopesError) -> AsterError {
    match error {
        UserBanScopesError::Empty => AsterError::validation_error_code(
            AsterErrorCode::UserBanReasonInvalid,
            "ban scopes must not be empty",
        ),
        UserBanScopesError::Invalid => AsterError::validation_error_code(
            AsterErrorCode::UserBanReasonInvalid,
            "ban scopes are invalid",
        ),
    }
}

fn is_effective(ban: &user_ban::Model, now: DateTime<Utc>) -> bool {
    ban.status.is_active()
        && ban.revoked_at.is_none()
        && ban.starts_at <= now
        && ban.expires_at.is_none_or(|expires_at| expires_at > now)
}

fn validate_time_range(starts_at: DateTime<Utc>, expires_at: Option<DateTime<Utc>>) -> Result<()> {
    if let Some(expires_at) = expires_at
        && expires_at <= starts_at
    {
        return Err(AsterError::validation_error_code(
            AsterErrorCode::UserBanDurationInvalid,
            "ban expires_at must be later than starts_at",
        ));
    }
    Ok(())
}

fn normalize_required_text(
    value: String,
    max_chars: usize,
    code: AsterErrorCode,
    label: &str,
) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > max_chars {
        return Err(AsterError::validation_error_code(
            code,
            format!("{label} must be 1-{max_chars} characters"),
        ));
    }
    Ok(value.to_string())
}

fn normalize_optional_text(
    value: Option<String>,
    max_chars: usize,
    code: AsterErrorCode,
    label: &str,
) -> Result<Option<String>> {
    match value {
        Some(value) => {
            let value = value.trim();
            if value.is_empty() {
                return Ok(None);
            }
            if value.chars().count() > max_chars {
                return Err(AsterError::validation_error_code(
                    code,
                    format!("{label} must not exceed {max_chars} characters"),
                ));
            }
            Ok(Some(value.to_string()))
        }
        None => Ok(None),
    }
}

fn normalize_nullable_text_patch(
    value: Option<NullablePatch<String>>,
    max_chars: usize,
    code: AsterErrorCode,
    label: &str,
) -> Result<Option<Option<String>>> {
    match value {
        Some(NullablePatch::Null) => Ok(Some(None)),
        Some(NullablePatch::Value(value)) => {
            normalize_optional_text(Some(value), max_chars, code, label).map(Some)
        }
        Some(NullablePatch::Absent) | None => Ok(None),
    }
}
