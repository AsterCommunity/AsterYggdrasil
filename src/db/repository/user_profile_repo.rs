//! User profile repository.

use crate::errors::{AsterError, MapAsterErr, Result};
use aster_yggdrasil_model::entities::user_profile::{self, Entity as UserProfile};
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use std::collections::HashMap;

pub async fn find_by_user_id<C: ConnectionTrait>(
    db: &C,
    user_id: i64,
) -> Result<Option<user_profile::Model>> {
    UserProfile::find_by_id(user_id)
        .one(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

pub async fn find_by_user_ids<C: ConnectionTrait>(
    db: &C,
    user_ids: &[i64],
) -> Result<HashMap<i64, user_profile::Model>> {
    if user_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let rows = UserProfile::find()
        .filter(user_profile::Column::UserId.is_in(user_ids.iter().copied()))
        .all(db)
        .await
        .map_aster_err(AsterError::database_operation)?;

    Ok(rows.into_iter().map(|row| (row.user_id, row)).collect())
}

pub async fn create<C: ConnectionTrait>(
    db: &C,
    model: user_profile::ActiveModel,
) -> Result<user_profile::Model> {
    model
        .insert(db)
        .await
        .map_aster_err(AsterError::database_operation)
}

pub async fn update<C: ConnectionTrait>(
    db: &C,
    model: user_profile::ActiveModel,
) -> Result<user_profile::Model> {
    model
        .update(db)
        .await
        .map_aster_err(AsterError::database_operation)
}
