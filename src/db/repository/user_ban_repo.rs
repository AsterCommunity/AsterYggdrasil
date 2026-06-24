//! Repository helpers for user capability bans.

use crate::entities::user_ban::{self, Entity as UserBan};
use crate::entities::user_ban_event::{self, Entity as UserBanEvent};
use crate::errors::{AsterError, MapAsterErr, Result};
use crate::types::{UserBanEventType, UserBanScope, UserBanScopes, UserBanStatus};
use aster_forge_api::CursorSlice;
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, EntityTrait, IntoActiveModel,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};

#[derive(Debug, Clone)]
pub struct CreateUserBan {
    pub user_id: i64,
    pub scopes: UserBanScopes,
    pub reason: String,
    pub public_reason: Option<String>,
    pub admin_note: Option<String>,
    pub created_by_user_id: Option<i64>,
    pub starts_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateUserBan {
    pub scopes: Option<UserBanScopes>,
    pub reason: Option<String>,
    pub public_reason: Option<Option<String>>,
    pub admin_note: Option<Option<String>>,
    pub starts_at: Option<DateTime<Utc>>,
    pub expires_at: Option<Option<DateTime<Utc>>>,
}

#[derive(Debug, Clone)]
pub struct CreateUserBanEvent {
    pub ban_id: i64,
    pub actor_user_id: Option<i64>,
    pub event_type: UserBanEventType,
    pub previous_status: Option<UserBanStatus>,
    pub next_status: Option<UserBanStatus>,
    pub previous_scopes: Option<UserBanScopes>,
    pub next_scopes: Option<UserBanScopes>,
    pub previous_expires_at: Option<DateTime<Utc>>,
    pub next_expires_at: Option<DateTime<Utc>>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UserBanListFilter {
    pub user_id: Option<i64>,
    pub status: Option<UserBanStatus>,
    pub effective_only: bool,
}

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

pub async fn find_by_id<C: ConnectionTrait>(db: &C, id: i64) -> Result<Option<user_ban::Model>> {
    UserBan::find_by_id(id)
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

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
