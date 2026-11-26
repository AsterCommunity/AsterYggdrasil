//! Repository helpers for user capability bans.

use crate::errors::{AsterError, MapAsterErr, Result};
use aster_forge_api::CursorSlice;
use aster_yggdrasil_model::entities::user_ban::{self, Entity as UserBan};
use aster_yggdrasil_model::entities::user_ban_event::{self, Entity as UserBanEvent};
use aster_yggdrasil_model::types::{
    user::UserBanEventType, user::UserBanScope, user::UserBanScopes, user::UserBanStatus,
};
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};

/// Input for creating a capability ban against a user.
#[derive(Debug, Clone)]
pub struct CreateUserBan {
    /// Banned user.
    pub user_id: i64,
    /// Capability scopes the ban removes.
    pub scopes: UserBanScopes,
    /// Internal reason shown to admins.
    pub reason: String,
    /// Reason shown to the affected user when set.
    pub public_reason: Option<String>,
    /// Free-form note for admin bookkeeping.
    pub admin_note: Option<String>,
    /// Admin or system actor issuing the ban; `None` means automated.
    pub created_by_user_id: Option<i64>,
    /// Instant the ban becomes effective (may be in the future).
    pub starts_at: DateTime<Utc>,
    /// Expiry instant for temporary bans; `None` makes the ban permanent.
    pub expires_at: Option<DateTime<Utc>>,
}

/// Partial update for a ban; `None` fields are left unchanged.
///
/// Double-`Option` fields distinguish clearing a value (`Some(None)`) from leaving it alone
/// (`None`).
#[derive(Debug, Clone, Default)]
pub struct UpdateUserBan {
    /// Replaces the banned scopes when set.
    pub scopes: Option<UserBanScopes>,
    /// Replaces the internal reason when set.
    pub reason: Option<String>,
    /// Replaces or clears the public reason when set.
    pub public_reason: Option<Option<String>>,
    /// Replaces or clears the admin note when set.
    pub admin_note: Option<Option<String>>,
    /// Replaces the effective start instant when set.
    pub starts_at: Option<DateTime<Utc>>,
    /// Replaces or clears the expiry (clearing makes the ban permanent) when set.
    pub expires_at: Option<Option<DateTime<Utc>>>,
}

/// Audit trail entry describing a state change on a ban.
#[derive(Debug, Clone)]
pub struct CreateUserBanEvent {
    /// Ban the event belongs to.
    pub ban_id: i64,
    /// Admin or system actor causing the change; `None` means automated.
    pub actor_user_id: Option<i64>,
    /// Kind of change (created, updated, revoked, expired, ...).
    pub event_type: UserBanEventType,
    /// Status before the change when it transitioned.
    pub previous_status: Option<UserBanStatus>,
    /// Status after the change when it transitioned.
    pub next_status: Option<UserBanStatus>,
    /// Scopes before the change when they changed.
    pub previous_scopes: Option<UserBanScopes>,
    /// Scopes after the change when they changed.
    pub next_scopes: Option<UserBanScopes>,
    /// Expiry before the change when it changed.
    pub previous_expires_at: Option<DateTime<Utc>>,
    /// Expiry after the change when it changed.
    pub next_expires_at: Option<DateTime<Utc>>,
    /// Free-form note attached to the event.
    pub note: Option<String>,
}

/// Optional filters for the admin ban list; all set filters combine with AND.
#[derive(Debug, Clone, Default)]
pub struct UserBanListFilter {
    /// Restricts results to bans against this user when set.
    pub user_id: Option<i64>,
    /// Restricts results to one ban status when set.
    pub status: Option<UserBanStatus>,
    /// When true, only returns bans currently in force (active window at query time).
    pub effective_only: bool,
}

/// Inserts a new ban in `Active` status.
pub async fn create<C: ConnectionTrait>(db: &C, input: CreateUserBan) -> Result<user_ban::Model> {
    let now = Utc::now();
    user_ban::ActiveModel {
        user_id: Set(input.user_id),
        scopes: Set(input.scopes),
        status: Set(UserBanStatus::Active),
        reason: Set(input.reason),
        public_reason: Set(input.public_reason),
        admin_note: Set(input.admin_note),
        created_by_user_id: Set(input.created_by_user_id),
        starts_at: Set(input.starts_at),
        expires_at: Set(input.expires_at),
        revoked_at: Set(None),
        revoked_by_user_id: Set(None),
        revoke_note: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_aster_err(AsterError::database_operation)
}

/// Loads a ban by id.
///
/// Returns `None` when no ban with that id exists.
pub async fn find_by_id<C: ConnectionTrait>(db: &C, id: i64) -> Result<Option<user_ban::Model>> {
    UserBan::find_by_id(id)
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

/// Finds a ban against a user currently in force for one capability scope.
///
/// "Effective" means active status with `starts_at` reached and expiry not passed at `now`.
/// The newest effective ban covering the scope wins; returns `None` when the scope is clean.
pub async fn find_effective_for_scope<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    scope: UserBanScope,
    now: DateTime<Utc>,
) -> Result<Option<user_ban::Model>> {
    let bans = UserBan::find()
        .filter(user_ban::Column::UserId.eq(user_id))
        .filter(effective_condition(now))
        .order_by_desc(user_ban::Column::CreatedAt)
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(bans.into_iter().find(|ban| ban.scopes.contains(scope)))
}

/// Lists a user's bans currently in force at `now`, newest created first.
pub async fn list_effective_for_user<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
    now: DateTime<Utc>,
) -> Result<Vec<user_ban::Model>> {
    UserBan::find()
        .filter(user_ban::Column::UserId.eq(user_id))
        .filter(effective_condition(now))
        .order_by_desc(user_ban::Column::CreatedAt)
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

/// Paginates bans using a keyset cursor over `(created_at, id)`, newest first, with the given
/// filters.
///
/// `limit` is clamped to 1..=100; `after` is the cursor of the last item of the previous page.
/// The returned [`CursorSlice`] carries the filtered total and a next cursor, which is absent
/// on the last page.
pub async fn list_cursor<C: ConnectionTrait>(
    db: &C,
    limit: u64,
    filter: UserBanListFilter,
    after: Option<(DateTime<Utc>, i64)>,
) -> Result<CursorSlice<user_ban::Model>> {
    let limit = limit.clamp(1, 100);
    let mut query = filtered_query(filter);
    let total = query
        .clone()
        .count(db)
        .await
        .map_aster_err(AsterError::database_operation)?;

    if let Some((created_at, id)) = after {
        query = query.filter(
            Condition::any()
                .add(user_ban::Column::CreatedAt.lt(created_at))
                .add(
                    Condition::all()
                        .add(user_ban::Column::CreatedAt.eq(created_at))
                        .add(user_ban::Column::Id.lt(id)),
                ),
        );
    }

    let items = query
        .order_by_desc(user_ban::Column::CreatedAt)
        .order_by_desc(user_ban::Column::Id)
        .limit(limit.saturating_add(1))
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)?;
    Ok(CursorSlice::from_overfetch(items, total, limit)?)
}

/// Applies the set fields of the update to a ban row, stamping `updated_at`.
pub async fn update<C: ConnectionTrait>(
    db: &C,
    ban: user_ban::Model,
    input: UpdateUserBan,
) -> Result<user_ban::Model> {
    let mut active = ban.into_active_model();
    if let Some(scopes) = input.scopes {
        active.scopes = Set(scopes);
    }
    if let Some(reason) = input.reason {
        active.reason = Set(reason);
    }
    if let Some(public_reason) = input.public_reason {
        active.public_reason = Set(public_reason);
    }
    if let Some(admin_note) = input.admin_note {
        active.admin_note = Set(admin_note);
    }
    if let Some(starts_at) = input.starts_at {
        active.starts_at = Set(starts_at);
    }
    if let Some(expires_at) = input.expires_at {
        active.expires_at = Set(expires_at);
    }
    active.updated_at = Set(Utc::now());
    active
        .update(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

/// Transitions a ban to `Revoked`, recording who revoked it, when, and why.
pub async fn revoke<C: ConnectionTrait>(
    db: &C,
    ban: user_ban::Model,
    revoked_by_user_id: Option<i64>,
    revoke_note: Option<String>,
    revoked_at: DateTime<Utc>,
) -> Result<user_ban::Model> {
    let mut active = ban.into_active_model();
    active.status = Set(UserBanStatus::Revoked);
    active.revoked_at = Set(Some(revoked_at));
    active.revoked_by_user_id = Set(revoked_by_user_id);
    active.revoke_note = Set(revoke_note);
    active.updated_at = Set(Utc::now());
    active
        .update(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

/// Appends an audit event to a ban's history.
pub async fn create_event<C: ConnectionTrait>(
    db: &C,
    input: CreateUserBanEvent,
) -> Result<user_ban_event::Model> {
    user_ban_event::ActiveModel {
        ban_id: Set(input.ban_id),
        actor_user_id: Set(input.actor_user_id),
        event_type: Set(input.event_type),
        previous_status: Set(input.previous_status),
        next_status: Set(input.next_status),
        previous_scopes: Set(input.previous_scopes),
        next_scopes: Set(input.next_scopes),
        previous_expires_at: Set(input.previous_expires_at),
        next_expires_at: Set(input.next_expires_at),
        note: Set(input.note),
        created_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_aster_err(AsterError::database_operation)
}

/// Lists a ban's audit events, newest first.
pub async fn list_events<C: ConnectionTrait>(
    db: &C,
    ban_id: i64,
) -> Result<Vec<user_ban_event::Model>> {
    UserBanEvent::find()
        .filter(user_ban_event::Column::BanId.eq(ban_id))
        .order_by_desc(user_ban_event::Column::CreatedAt)
        .order_by_desc(user_ban_event::Column::Id)
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

fn filtered_query(filter: UserBanListFilter) -> sea_orm::Select<UserBan> {
    let mut query = UserBan::find();
    if let Some(user_id) = filter.user_id {
        query = query.filter(user_ban::Column::UserId.eq(user_id));
    }
    if let Some(status) = filter.status {
        query = query.filter(user_ban::Column::Status.eq(status));
    }
    if filter.effective_only {
        query = query.filter(effective_condition(Utc::now()));
    }
    query
}

fn effective_condition(now: DateTime<Utc>) -> Condition {
    Condition::all()
        .add(user_ban::Column::Status.eq(UserBanStatus::Active))
        .add(user_ban::Column::StartsAt.lte(now))
        .add(
            Condition::any()
                .add(user_ban::Column::ExpiresAt.is_null())
                .add(user_ban::Column::ExpiresAt.gt(now)),
        )
}
