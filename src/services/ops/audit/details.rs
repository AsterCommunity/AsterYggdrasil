use chrono::{DateTime, Utc};
use serde::Serialize;

use aster_forge_config::ConfigVisibility;
use aster_yggdrasil_model::types::user::{UserBanScope, UserBanStatus, UserRole, UserStatus};
use aster_yggdrasil_model::types::yggdrasil::{
    MinecraftTextureLibraryStatus, MinecraftTextureModel, MinecraftTextureReportReason,
    MinecraftTextureReportStatus, MinecraftTextureType,
};
use aster_yggdrasil_model::types::{task::BackgroundTaskKind, task::BackgroundTaskStatus};

/// Audit payload for a runtime config value update. `value` is the
/// audit-safe rendering of the stored value: sensitive configs are redacted
/// before reaching this struct.
#[derive(Serialize)]
pub struct ConfigUpdateDetails<'a> {
    /// Audit-safe rendering of the new stored value (redacted when sensitive).
    pub value: &'a str,
    /// Visibility level of the config after the update.
    pub visibility: ConfigVisibility,
    /// Visibility level before the update, when it changed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prior_visibility: Option<ConfigVisibility>,
}

/// Audit payload for a runtime config action execution (test mail, captcha
/// preview, Yggdrasil key rotation).
#[derive(Serialize)]
pub struct ConfigActionDetails<'a> {
    /// Action identifier that was executed.
    pub action: &'a str,
    /// Recipient of a test email, when the action targeted one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_email: Option<&'a str>,
}

/// Audit payload for one outbound mail send driven by the persistent outbox.
#[derive(Serialize)]
pub struct MailAuditDetails<'a> {
    /// Recipient address.
    pub to_address: &'a str,
    /// Rendered mail template identifier.
    pub template_code: &'a str,
    /// Recipient display name, when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_name: Option<&'a str>,
    /// Final subject line, when rendering produced one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<&'a str>,
    /// Outbox row this send belongs to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outbox_id: Option<i64>,
    /// Delivery attempts made before this record, when retried.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt_count: Option<i32>,
    /// Delivery failure reason, when the send failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<&'a str>,
}

/// Audit payload for the admin background-task cleanup run.
#[derive(Serialize)]
pub struct AdminTaskCleanupAuditDetails {
    /// Task records removed by this run.
    pub removed: u64,
    /// Cutoff applied: tasks finished before this instant were eligible.
    pub finished_before: DateTime<Utc>,
    /// Task kind restriction passed to the cleanup, when set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<BackgroundTaskKind>,
    /// Task status restriction passed to the cleanup, when set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<BackgroundTaskStatus>,
}

/// Audit payload for the admin retry of a failed background task.
#[derive(Serialize)]
pub struct TaskRetryAuditDetails {
    /// Wire name of the retried task kind.
    pub kind: String,
    /// Attempt count accumulated before the retry reset the schedule.
    pub previous_attempt_count: i32,
}

/// Audit payload for a successful credential login.
#[derive(Serialize)]
pub struct LoginAuditDetails<'a> {
    /// Identifier the user authenticated with (username or email).
    pub identifier: &'a str,
}

/// Audit payload for auth session lifecycle events (revocation of one, all,
/// or the other sessions of a user).
#[derive(Serialize)]
pub struct AuthSessionAuditDetails<'a> {
    /// Session affected by a single-session operation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<&'a str>,
    /// Number of sessions removed by a bulk revocation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub removed: Option<u64>,
    /// Whether the calling session itself was revoked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_current: Option<bool>,
}

/// Audit payload for passkey lifecycle events (registration, rename, delete,
/// authentication).
#[derive(Serialize)]
pub struct PasskeyAuditDetails<'a> {
    /// Passkey credential row the event concerns.
    pub passkey_id: i64,
    /// Credential label after the event, when one applies.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<&'a str>,
    /// Credential label before a rename.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_name: Option<&'a str>,
    /// Credential label after a rename.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_name: Option<&'a str>,
    /// WebAuthn backup eligibility flag reported by the authenticator.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_eligible: Option<bool>,
    /// WebAuthn backup state flag reported by the authenticator.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backed_up: Option<bool>,
    /// WebAuthn signature counter observed at this use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sign_count: Option<i64>,
    /// Previous successful-use timestamp, when recorded on the credential.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_used_at: Option<DateTime<Utc>>,
}

/// Audit payload for user account creation and admin account changes;
/// captures the account state after the change.
#[derive(Serialize)]
pub struct UserAuditDetails<'a> {
    /// Username after the change.
    pub username: &'a str,
    /// Account email after the change.
    pub email: &'a str,
    /// Account role after the change.
    pub role: UserRole,
    /// Account status after the change.
    pub status: UserStatus,
    /// Whether the account is flagged to change password at next login.
    pub must_change_password: bool,
    /// Whether this admin action generated a temporary password.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temporary_password_generated: Option<bool>,
    /// Minecraft profiles bound to the account at audit time.
    pub profile_count: u64,
    /// Live auth sessions of the account at audit time.
    pub active_session_count: u64,
}

/// Audit payload for a bulk revocation of a user's auth sessions.
#[derive(Serialize)]
pub struct UserSessionRevokeAuditDetails {
    /// Sessions removed by the revocation.
    pub removed: u64,
}

/// Audit payload for applying or revoking a user ban.
#[derive(Serialize)]
pub struct UserBanAuditDetails<'a> {
    /// Banned user's ID.
    pub target_user_id: i64,
    /// Account surfaces the ban applies to (login, texture upload, ...).
    pub scopes: &'a [UserBanScope],
    /// Ban status as stored.
    pub status: UserBanStatus,
    /// Stored status with expired-but-active bans folded to `Expired` at
    /// audit time.
    pub effective_status: UserBanStatus,
    /// Internal ban reason recorded by the issuer.
    pub reason: &'a str,
    /// Reason shown to the banned user, when provided.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_reason: Option<&'a str>,
    /// Issuer-only note, when provided.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub admin_note: Option<&'a str>,
    /// Note recorded when lifting the ban early.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoke_note: Option<&'a str>,
    /// Instant the ban takes effect (UTC).
    pub starts_at: DateTime<Utc>,
    /// Instant the ban lapses, when scheduled (UTC).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}

/// Audit payload for Minecraft profile creation and deletion.
#[derive(Serialize)]
pub struct MinecraftProfileAuditDetails<'a> {
    /// Affected profile UUID.
    pub profile_uuid: &'a str,
    /// Profile name at audit time.
    pub profile_name: &'a str,
    /// Texture records removed alongside a profile deletion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_texture_count: Option<usize>,
    /// Yggdrasil tokens invalidated alongside a profile deletion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_token_count: Option<u64>,
}

/// Audit payload for a controlled Minecraft profile rename; token
/// invalidation lets launchers pick up the new name via refresh.
#[derive(Serialize)]
pub struct MinecraftProfileRenameAuditDetails<'a> {
    /// Renamed profile UUID; unchanged across renames.
    pub profile_uuid: &'a str,
    /// Name before the rename.
    pub old_profile_name: &'a str,
    /// Name after the rename.
    pub new_profile_name: &'a str,
    /// Yggdrasil tokens temporarily invalidated by the rename.
    pub temporarily_invalidated_token_count: u64,
}

/// Audit payload for Minecraft texture upload, bind, and removal events.
#[derive(Serialize)]
pub struct MinecraftTextureAuditDetails<'a> {
    /// Profile the texture is bound to.
    pub profile_uuid: &'a str,
    /// Profile name at audit time.
    pub profile_name: &'a str,
    /// Texture slot (skin, cape, ...).
    pub texture_type: MinecraftTextureType,
    /// Content hash of the stored texture, when uploaded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture_hash: Option<&'a str>,
    /// Skin model the texture is rendered with, when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture_model: Option<MinecraftTextureModel>,
    /// Image width in pixels, when measured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,
    /// Image height in pixels, when measured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,
    /// Stored file size in bytes, when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size: Option<i64>,
    /// Texture library listing status after the event, when the texture is
    /// library-visible.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub library_status: Option<MinecraftTextureLibraryStatus>,
    /// Reviewer note attached to the texture, when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_note: Option<&'a str>,
}

/// Audit payload for texture library report resolution (approval, rejection,
/// takedown).
#[derive(Serialize)]
pub struct MinecraftTextureReportAuditDetails {
    /// Reported texture record ID.
    pub texture_id: i64,
    /// Report row this resolution closes.
    pub report_id: i64,
    /// Reason the report was filed with.
    pub reason: MinecraftTextureReportReason,
    /// Report status after the resolution.
    pub report_status: MinecraftTextureReportStatus,
    /// Texture library status applied by the resolution, when changed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub library_status: Option<MinecraftTextureLibraryStatus>,
}

/// Audit payload for the Yggdrasil `authenticate` endpoint.
#[derive(Serialize)]
pub struct YggdrasilAuthenticateAuditDetails<'a> {
    /// Account identifier the launcher authenticated with.
    pub identifier: &'a str,
    /// Profile the launcher selected, when it owns one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_profile_uuid: Option<&'a str>,
    /// Name of the selected profile, when it owns one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_profile_name: Option<&'a str>,
    /// Number of profiles available for selection on the account.
    pub available_profile_count: usize,
}

/// Audit payload for Yggdrasil token lifecycle events (refresh, invalidate,
/// signout).
#[derive(Serialize)]
pub struct YggdrasilTokenAuditDetails<'a> {
    /// Profile the token was bound to, when identified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_uuid: Option<&'a str>,
    /// Name of that profile, when identified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_name: Option<&'a str>,
}

/// Audit payload for the Yggdrasil `join` endpoint (server login handshake).
#[derive(Serialize)]
pub struct YggdrasilJoinAuditDetails<'a> {
    /// Profile joining the server.
    pub profile_uuid: &'a str,
    /// Name of the joining profile.
    pub profile_name: &'a str,
    /// Hash of the server's join challenge, per protocol semantics.
    pub server_id_hash: &'a str,
}

/// Audit payload capturing the full configuration of one session-forward
/// upstream server at change time.
#[derive(Serialize)]
pub struct YggdrasilSessionForwardServerAuditDetails<'a> {
    /// Upstream kind (remote Mojang, builtin, custom provider).
    pub provider_kind: &'a str,
    /// Session endpoint this server proxies (session server, textures, ...).
    pub endpoint_kind: &'a str,
    /// Upstream base URL; absent for builtin providers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<&'a str>,
    /// Whether the entry is a builtin default server.
    pub builtin: bool,
    /// Whether the server participates in request forwarding.
    pub enabled: bool,
    /// Higher values are tried first when selecting an upstream.
    pub priority: i32,
    /// Relative share of requests among same-priority servers.
    pub weight: i32,
    /// Upstream request timeout in milliseconds.
    pub timeout_ms: i32,
    /// Whether texture requests are also forwarded to this server.
    pub texture_forward_enabled: bool,
}

/// Audit payload for one session-forward availability check against an
/// upstream server.
#[derive(Serialize)]
pub struct YggdrasilSessionForwardCheckAuditDetails<'a> {
    /// Username the check authenticated as.
    pub username: &'a str,
    /// Hash of the server challenge used in the check handshake.
    pub server_id_hash: &'a str,
    /// Upstream server row the check ran against.
    pub upstream_id: i64,
    /// Display name of that upstream server.
    pub upstream_name: &'a str,
    /// Upstream kind of that server.
    pub provider_kind: &'a str,
    /// Session endpoint kind probed.
    pub endpoint_kind: &'a str,
    /// Check outcome: `matched`, `no_match`, or `failed`.
    pub result: &'a str,
    /// Whether texture forwarding is enabled on the probed server.
    pub texture_forward_enabled: bool,
    /// Profile resolved during the check, when one matched.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_uuid: Option<&'a str>,
    /// Failure detail, when the check failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<&'a str>,
}

/// Audit payload for the admin external auth provider parameter test run;
/// `provider` and `key` are draft placeholders because the test runs on an
/// unsaved provider configuration.
#[derive(Serialize)]
pub struct ExternalAuthProviderTestParamsAuditDetails<'a> {
    /// Provider under test; "draft" for unsaved configurations.
    pub provider: &'a str,
    /// Parameter key under test; "draft" for unsaved configurations.
    pub key: &'a str,
    /// Whether the parameter test succeeded.
    pub success: bool,
}

/// Serializes audit details to JSON, mapping serialization failure to `None`
/// with a warning instead of failing the audited operation.
pub fn details<T: Serialize>(value: T) -> Option<serde_json::Value> {
    match serde_json::to_value(value) {
        Ok(value) => Some(value),
        Err(error) => {
            tracing::warn!("failed to serialize audit details: {error}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::{
        AdminTaskCleanupAuditDetails, ConfigActionDetails, ConfigUpdateDetails, LoginAuditDetails,
        MailAuditDetails, TaskRetryAuditDetails, UserAuditDetails, UserSessionRevokeAuditDetails,
        details,
    };
    use aster_forge_config::ConfigVisibility;
    use aster_yggdrasil_model::types::{
        task::BackgroundTaskKind, task::BackgroundTaskStatus, user::UserRole, user::UserStatus,
    };
    #[test]
    fn details_serializes_config_update_and_omits_missing_prior_visibility() {
        assert_eq!(
            details(ConfigUpdateDetails {
                value: "public title",
                visibility: ConfigVisibility::Public,
                prior_visibility: None,
            })
            .unwrap(),
            serde_json::json!({
                "value": "public title",
                "visibility": "public",
            })
        );

        assert_eq!(
            details(ConfigUpdateDetails {
                value: "***REDACTED***",
                visibility: ConfigVisibility::Authenticated,
                prior_visibility: Some(ConfigVisibility::Private),
            })
            .unwrap(),
            serde_json::json!({
                "value": "***REDACTED***",
                "visibility": "authenticated",
                "prior_visibility": "private",
            })
        );
    }

    #[test]
    fn details_serializes_config_action_and_omits_missing_target_email() {
        assert_eq!(
            details(ConfigActionDetails {
                action: "send_test_email",
                target_email: Some("admin@example.com"),
            })
            .unwrap(),
            serde_json::json!({
                "action": "send_test_email",
                "target_email": "admin@example.com",
            })
        );

        assert_eq!(
            details(ConfigActionDetails {
                action: "send_test_email",
                target_email: None,
            })
            .unwrap(),
            serde_json::json!({
                "action": "send_test_email",
            })
        );
    }

    #[test]
    fn details_serializes_task_cleanup_and_retry_shapes() {
        let finished_before = Utc::now();
        assert_eq!(
            details(AdminTaskCleanupAuditDetails {
                removed: 3,
                finished_before,
                kind: Some(BackgroundTaskKind::SystemRuntime),
                status: Some(BackgroundTaskStatus::Failed),
            })
            .unwrap(),
            serde_json::json!({
                "removed": 3,
                "finished_before": finished_before,
                "kind": "system_runtime",
                "status": "failed",
            })
        );

        assert_eq!(
            details(TaskRetryAuditDetails {
                kind: "system_runtime".to_string(),
                previous_attempt_count: 2,
            })
            .unwrap(),
            serde_json::json!({
                "kind": "system_runtime",
                "previous_attempt_count": 2,
            })
        );
    }

    #[test]
    fn details_serializes_mail_audit_shape() {
        assert_eq!(
            details(MailAuditDetails {
                to_address: "user@example.com",
                template_code: "password_reset",
                to_name: Some("User"),
                subject: Some("Reset password"),
                outbox_id: Some(42),
                attempt_count: Some(2),
                error: Some("smtp timeout"),
            })
            .unwrap(),
            serde_json::json!({
                "to_address": "user@example.com",
                "template_code": "password_reset",
                "to_name": "User",
                "subject": "Reset password",
                "outbox_id": 42,
                "attempt_count": 2,
                "error": "smtp timeout",
            })
        );
    }

    #[test]
    fn details_serializes_login_identifier() {
        assert_eq!(
            details(LoginAuditDetails {
                identifier: "admin@example.com",
            })
            .unwrap(),
            serde_json::json!({ "identifier": "admin@example.com" })
        );
    }

    #[test]
    fn details_serializes_user_audit_shapes() {
        assert_eq!(
            details(UserAuditDetails {
                username: "alex",
                email: "alex@example.com",
                role: UserRole::Admin,
                status: UserStatus::Active,
                must_change_password: true,
                temporary_password_generated: Some(false),
                profile_count: 2,
                active_session_count: 3,
            })
            .unwrap(),
            serde_json::json!({
                "username": "alex",
                "email": "alex@example.com",
                "role": "admin",
                "status": "active",
                "must_change_password": true,
                "temporary_password_generated": false,
                "profile_count": 2,
                "active_session_count": 3,
            })
        );

        assert_eq!(
            details(UserSessionRevokeAuditDetails { removed: 4 }).unwrap(),
            serde_json::json!({ "removed": 4 })
        );
    }
}
