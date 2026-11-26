//! User invitation registration service.

use chrono::{DateTime, Duration, Utc};
use sea_orm::{ConnectionTrait, Set};
use serde::Serialize;
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

use crate::api::error_code::AsterErrorCode;
use crate::config::{auth_runtime, branding, local_email_policy::LocalEmailPolicy, site_url};
use crate::db::repository::{user_invitation_repo, user_repo};
use crate::errors::{AsterError, Result};
use crate::runtime::{DatabaseRuntimeState, RuntimeConfigRuntimeState};
use crate::services::{
    auth::local::shared::{CreateUserWithRoleInput, create_user_with_role},
    mail::template::MailTemplatePayload,
};
use aster_forge_api::{CursorPage, DateTimeIdCursor};
use aster_forge_crypto as hash;
use aster_forge_utils::numbers::u64_to_i64;
use aster_yggdrasil_model::entities::{user, user_invitation};
use aster_yggdrasil_model::types::user::{UserInvitationStatus, UserRole, UserStatus};

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
/// Administrator-facing view of an invitation.
pub struct AdminUserInvitationInfo {
    /// Invitation record identifier.
    pub id: i64,
    /// Invited email address.
    pub email: String,
    /// Lifecycle status (`pending`, `accepted`, `expired` or `revoked`).
    pub status: UserInvitationStatus,
    /// Administrator who issued the invitation.
    pub invited_by: i64,
    /// Account created by accepting the invitation, once accepted.
    pub accepted_user_id: Option<i64>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    /// Time the invitation stops being usable.
    pub expires_at: DateTime<Utc>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    /// Invitation creation time.
    pub created_at: DateTime<Utc>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    /// Invitation last-update time.
    pub updated_at: DateTime<Utc>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    /// Time the invitation was accepted, if accepted.
    pub accepted_at: Option<DateTime<Utc>>,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = Option<String>))]
    /// Time the invitation was revoked, if revoked.
    pub revoked_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Registration URL embedding the plaintext token; returned only by the
    /// create call, never from listings.
    pub invitation_url: Option<String>,
    /// Whether the invitation mail was enqueued for delivery.
    pub mail_queued: bool,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
/// Minimal invitation details shown to the token holder before registering.
pub struct PublicUserInvitationInfo {
    /// Email address the invitation was issued for; fixed at registration.
    pub email: String,
    #[cfg_attr(all(debug_assertions, feature = "openapi"), schema(value_type = String))]
    /// Time the invitation stops being usable.
    pub expires_at: DateTime<Utc>,
}

/// Issues an invitation for an email address after local email policy
/// checks: revokes any still-pending invitations for the same address,
/// stores a new pending invitation whose TTL comes from the runtime config,
/// and enqueues the invitation mail, returning the one-time invitation URL.
pub async fn create_invitation<S>(
    state: &S,
    email: &str,
    invited_by: i64,
) -> Result<AdminUserInvitationInfo>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    let email = aster_forge_validation::email::normalize_email(email)?;
    LocalEmailPolicy::from_runtime_config(state.runtime_config()).check(&email)?;

    let token = aster_forge_utils::id::new_short_token();
    let token_hash = hash::sha256_hex(token.as_bytes());
    let now = Utc::now();
    let ttl_secs = u64_to_i64(
        auth_runtime::user_invitation_ttl_secs(state.runtime_config()),
        "user invitation ttl",
    )?;
    let expires_at = now + Duration::seconds(ttl_secs);
    let invitation_url = invitation_url(state.runtime_config(), &token);
    let expires_in = format_mail_duration_seconds(ttl_secs);
    let site_name = branding::title_or_default(state.runtime_config());

    let invitation = crate::db::transaction::with_transaction(state.writer_db(), async |txn| {
        ensure_email_available(txn, &email).await?;
        for existing in user_invitation_repo::find_pending_by_email(txn, &email).await? {
            user_invitation_repo::mark_revoked_if_pending(txn, existing.id).await?;
        }

        let invitation = user_invitation_repo::create(
            txn,
            user_invitation::ActiveModel {
                email: Set(email.clone()),
                token_hash: Set(token_hash),
                status: Set(UserInvitationStatus::Pending),
                invited_by: Set(invited_by),
                accepted_user_id: Set(None),
                expires_at: Set(expires_at),
                created_at: Set(now),
                updated_at: Set(now),
                accepted_at: Set(None),
                revoked_at: Set(None),
                ..Default::default()
            },
        )
        .await?;

        crate::services::mail::outbox::enqueue(
            txn,
            &email,
            None,
            MailTemplatePayload::user_invitation(&email, &invitation_url, &site_name, &expires_in),
        )
        .await?;

        Ok(invitation)
    })
    .await?;

    Ok(to_admin_info(invitation, Some(invitation_url), true))
}

/// Returns a cursor-paged listing of invitations, newest first, with pending
/// invitations past their expiry displayed as expired; the listing never
/// carries invitation URLs.
pub async fn list_invitations<S>(
    state: &S,
    limit: u64,
    cursor: Option<(chrono::DateTime<chrono::Utc>, i64)>,
) -> Result<CursorPage<AdminUserInvitationInfo, DateTimeIdCursor>>
where
    S: DatabaseRuntimeState,
{
    let limit = limit.clamp(1, 100);
    let page = user_invitation_repo::list_cursor_after(state.reader_db(), limit, cursor).await?;
    let next_cursor = if page.has_more {
        page.items.last().map(|invitation| DateTimeIdCursor {
            value: invitation.created_at,
            id: invitation.id,
        })
    } else {
        None
    };
    let items = page
        .items
        .into_iter()
        .map(invitation_list_view)
        .map(|item| to_admin_info(item, None, false))
        .collect();
    Ok(CursorPage::new(items, page.total, limit, next_cursor))
}

/// Revokes a still-pending invitation so its token can no longer register an
/// account; invitations that were already accepted, expired or revoked are
/// rejected with their current status.
pub async fn revoke_invitation<S>(state: &S, id: i64) -> Result<AdminUserInvitationInfo>
where
    S: DatabaseRuntimeState,
{
    let invitation = user_invitation_repo::find_by_id(state.writer_db(), id).await?;
    let invitation = refresh_expired_status(state.writer_db(), invitation).await?;
    if invitation.status != UserInvitationStatus::Pending {
        return Err(invitation_status_error(invitation.status));
    }

    if !user_invitation_repo::mark_revoked_if_pending(state.writer_db(), invitation.id).await? {
        return Err(current_invitation_status_error(state.writer_db(), invitation.id).await);
    }

    let invitation = user_invitation_repo::find_by_id(state.writer_db(), id).await?;
    Ok(to_admin_info(invitation, None, false))
}

/// Validates an invitation token and returns the invited email and expiry so
/// the registration page can preview the invitation; errors for invalid,
/// non-pending or expired tokens.
pub async fn verify_public_invitation<S>(state: &S, token: &str) -> Result<PublicUserInvitationInfo>
where
    S: DatabaseRuntimeState,
{
    let invitation = find_valid_invitation_by_token(state.writer_db(), token).await?;
    Ok(PublicUserInvitationInfo {
        email: invitation.email,
        expires_at: invitation.expires_at,
    })
}

/// Consumes a valid pending invitation to create the invited account: the
/// token must resolve to a pending, unexpired invitation whose email still
/// passes policy and is not registered; the created user starts active with
/// the email pre-verified, and the invitation is marked accepted in the same
/// transaction.
pub async fn accept_invitation<S>(
    state: &S,
    token: &str,
    username: &str,
    password: &str,
) -> Result<user::Model>
where
    S: DatabaseRuntimeState + RuntimeConfigRuntimeState,
{
    let token_hash = invitation_token_hash(token)?;
    crate::db::transaction::with_transaction(state.writer_db(), async |txn| {
        let Some(invitation) = user_invitation_repo::find_by_token_hash(txn, &token_hash).await?
        else {
            return Err(invitation_invalid_error());
        };
        let invitation = refresh_expired_status(txn, invitation).await?;
        ensure_invitation_pending(&invitation)?;
        ensure_invitation_not_expired(txn, &invitation).await?;
        LocalEmailPolicy::from_runtime_config(state.runtime_config()).check(&invitation.email)?;
        ensure_email_available(txn, &invitation.email).await?;

        let user = create_user_with_role(
            txn,
            state,
            CreateUserWithRoleInput {
                username,
                email: &invitation.email,
                password,
                role: UserRole::User,
                status: UserStatus::Active,
                must_change_password: false,
                email_verified_at: Some(Utc::now()),
            },
        )
        .await?;
        if !user_invitation_repo::mark_accepted_if_pending(txn, invitation.id, user.id).await? {
            return Err(current_invitation_status_error(txn, invitation.id).await);
        }
        Ok(user)
    })
    .await
}

async fn find_valid_invitation_by_token<C: ConnectionTrait>(
    db: &C,
    token: &str,
) -> Result<user_invitation::Model> {
    let token_hash = invitation_token_hash(token)?;
    let Some(invitation) = user_invitation_repo::find_by_token_hash(db, &token_hash).await? else {
        return Err(invitation_invalid_error());
    };
    let invitation = refresh_expired_status(db, invitation).await?;
    ensure_invitation_pending(&invitation)?;
    ensure_invitation_not_expired(db, &invitation).await?;
    Ok(invitation)
}

async fn refresh_expired_status<C: ConnectionTrait>(
    db: &C,
    mut invitation: user_invitation::Model,
) -> Result<user_invitation::Model> {
    if invitation.status == UserInvitationStatus::Pending && invitation.expires_at <= Utc::now() {
        if !user_invitation_repo::mark_expired_if_pending(db, invitation.id).await? {
            return user_invitation_repo::find_by_id(db, invitation.id).await;
        }
        invitation.status = UserInvitationStatus::Expired;
        invitation.updated_at = Utc::now();
    }
    Ok(invitation)
}

fn invitation_list_view(mut invitation: user_invitation::Model) -> user_invitation::Model {
    if invitation.status == UserInvitationStatus::Pending && invitation.expires_at <= Utc::now() {
        invitation.status = UserInvitationStatus::Expired;
    }
    invitation
}

async fn ensure_invitation_not_expired<C: ConnectionTrait>(
    db: &C,
    invitation: &user_invitation::Model,
) -> Result<()> {
    if invitation.expires_at > Utc::now() {
        return Ok(());
    }
    if !user_invitation_repo::mark_expired_if_pending(db, invitation.id).await? {
        return Err(current_invitation_status_error(db, invitation.id).await);
    }
    Err(invitation_status_error(UserInvitationStatus::Expired))
}

fn ensure_invitation_pending(invitation: &user_invitation::Model) -> Result<()> {
    if invitation.status.is_pending() {
        Ok(())
    } else {
        Err(invitation_status_error(invitation.status))
    }
}

async fn ensure_email_available<C: ConnectionTrait>(db: &C, email: &str) -> Result<()> {
    if user_repo::find_by_email(db, email).await?.is_some() {
        return Err(AsterError::validation_error_code(
            AsterErrorCode::AuthEmailExists,
            "email already exists",
        ));
    }
    Ok(())
}

fn invitation_token_hash(token: &str) -> Result<String> {
    let token = token.trim();
    if token.is_empty() {
        return Err(invitation_invalid_error());
    }
    Ok(hash::sha256_hex(token.as_bytes()))
}

fn invitation_url(runtime_config: &crate::config::RuntimeConfig, token: &str) -> String {
    site_url::public_app_url_or_path(runtime_config, &format!("/invite/{token}"))
}

fn format_mail_duration_seconds(total_secs: i64) -> String {
    let total_secs = total_secs.max(1);
    let (value, unit) = if total_secs >= 86_400 && total_secs % 86_400 == 0 {
        (total_secs / 86_400, "day")
    } else if total_secs >= 3_600 && total_secs % 3_600 == 0 {
        (total_secs / 3_600, "hour")
    } else if total_secs >= 60 {
        ((total_secs + 59) / 60, "minute")
    } else {
        (total_secs, "second")
    };
    let suffix = if value == 1 { "" } else { "s" };
    format!("{value} {unit}{suffix}")
}

fn to_admin_info(
    invitation: user_invitation::Model,
    invitation_url: Option<String>,
    mail_queued: bool,
) -> AdminUserInvitationInfo {
    AdminUserInvitationInfo {
        id: invitation.id,
        email: invitation.email,
        status: invitation.status,
        invited_by: invitation.invited_by,
        accepted_user_id: invitation.accepted_user_id,
        expires_at: invitation.expires_at,
        created_at: invitation.created_at,
        updated_at: invitation.updated_at,
        accepted_at: invitation.accepted_at,
        revoked_at: invitation.revoked_at,
        invitation_url,
        mail_queued,
    }
}

fn invitation_invalid_error() -> AsterError {
    AsterError::validation_error_code(
        AsterErrorCode::AuthInvitationInvalid,
        "invitation token is invalid",
    )
}

fn invitation_status_error(status: UserInvitationStatus) -> AsterError {
    let (code, message) = match status {
        UserInvitationStatus::Pending => (
            AsterErrorCode::AuthInvitationInvalid,
            "invitation is not usable",
        ),
        UserInvitationStatus::Accepted => (
            AsterErrorCode::AuthInvitationAccepted,
            "invitation has already been accepted",
        ),
        UserInvitationStatus::Expired => (
            AsterErrorCode::AuthInvitationExpired,
            "invitation has expired",
        ),
        UserInvitationStatus::Revoked => (
            AsterErrorCode::AuthInvitationRevoked,
            "invitation has been revoked",
        ),
    };
    AsterError::validation_error_code(code, message)
}

async fn current_invitation_status_error<C: ConnectionTrait>(db: &C, id: i64) -> AsterError {
    match user_invitation_repo::find_by_id(db, id).await {
        Ok(invitation) => invitation_status_error(invitation.status),
        Err(error) => error,
    }
}
