use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::types::{BackgroundTaskKind, BackgroundTaskStatus, SystemConfigVisibility};
use crate::types::{
    MinecraftTextureLibraryStatus, MinecraftTextureModel, MinecraftTextureReportReason,
    MinecraftTextureReportStatus, MinecraftTextureType,
};
use crate::types::{UserRole, UserStatus};

#[derive(Serialize)]
pub struct ConfigUpdateDetails<'a> {
    pub value: &'a str,
    pub visibility: SystemConfigVisibility,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prior_visibility: Option<SystemConfigVisibility>,
}

#[derive(Serialize)]
pub struct ConfigActionDetails<'a> {
    pub action: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_email: Option<&'a str>,
}

#[derive(Serialize)]
pub struct MailAuditDetails<'a> {
    pub to_address: &'a str,
    pub template_code: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outbox_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<&'a str>,
}

#[derive(Serialize)]
pub struct AdminTaskCleanupAuditDetails {
    pub removed: u64,
    pub finished_before: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<BackgroundTaskKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<BackgroundTaskStatus>,
}

#[derive(Serialize)]
pub struct TaskRetryAuditDetails {
    pub kind: String,
    pub previous_attempt_count: i32,
}

#[derive(Serialize)]
pub struct LoginAuditDetails<'a> {
    pub identifier: &'a str,
}

#[derive(Serialize)]
pub struct AuthSessionAuditDetails<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub removed: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_current: Option<bool>,
}

#[derive(Serialize)]
pub struct PasskeyAuditDetails<'a> {
    pub passkey_id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_eligible: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backed_up: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sign_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
pub struct UserAuditDetails<'a> {
    pub username: &'a str,
    pub email: &'a str,
    pub role: UserRole,
    pub status: UserStatus,
    pub must_change_password: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temporary_password_generated: Option<bool>,
    pub profile_count: u64,
    pub active_session_count: u64,
}

#[derive(Serialize)]
pub struct UserSessionRevokeAuditDetails {
    pub removed: u64,
}

#[derive(Serialize)]
pub struct MinecraftProfileAuditDetails<'a> {
    pub profile_uuid: &'a str,
    pub profile_name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_texture_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_token_count: Option<u64>,
}

#[derive(Serialize)]
pub struct MinecraftProfileRenameAuditDetails<'a> {
    pub profile_uuid: &'a str,
    pub old_profile_name: &'a str,
    pub new_profile_name: &'a str,
    pub temporarily_invalidated_token_count: u64,
}

#[derive(Serialize)]
pub struct MinecraftTextureAuditDetails<'a> {
    pub profile_uuid: &'a str,
    pub profile_name: &'a str,
    pub texture_type: MinecraftTextureType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture_hash: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture_model: Option<MinecraftTextureModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub library_status: Option<MinecraftTextureLibraryStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_note: Option<&'a str>,
}

#[derive(Serialize)]
pub struct MinecraftTextureReportAuditDetails {
    pub texture_id: i64,
    pub report_id: i64,
    pub reason: MinecraftTextureReportReason,
    pub report_status: MinecraftTextureReportStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub library_status: Option<MinecraftTextureLibraryStatus>,
}

#[derive(Serialize)]
pub struct YggdrasilAuthenticateAuditDetails<'a> {
    pub identifier: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_profile_uuid: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_profile_name: Option<&'a str>,
    pub available_profile_count: usize,
}

#[derive(Serialize)]
pub struct YggdrasilTokenAuditDetails<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_uuid: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_name: Option<&'a str>,
}

#[derive(Serialize)]
pub struct YggdrasilJoinAuditDetails<'a> {
    pub profile_uuid: &'a str,
    pub profile_name: &'a str,
    pub server_id_hash: &'a str,
}

#[derive(Serialize)]
pub struct YggdrasilSessionForwardServerAuditDetails<'a> {
    pub provider_kind: &'a str,
    pub endpoint_kind: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<&'a str>,
    pub builtin: bool,
    pub enabled: bool,
    pub priority: i32,
    pub weight: i32,
    pub timeout_ms: i32,
    pub texture_forward_enabled: bool,
}

#[derive(Serialize)]
pub struct YggdrasilSessionForwardCheckAuditDetails<'a> {
    pub username: &'a str,
    pub server_id_hash: &'a str,
    pub upstream_id: i64,
    pub upstream_name: &'a str,
    pub provider_kind: &'a str,
    pub endpoint_kind: &'a str,
    pub result: &'a str,
    pub texture_forward_enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_uuid: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<&'a str>,
}

#[derive(Serialize)]
pub struct ExternalAuthProviderTestParamsAuditDetails<'a> {
    pub provider: &'a str,
    pub key: &'a str,
    pub success: bool,
}

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
    use super::{
        AdminTaskCleanupAuditDetails, ConfigActionDetails, ConfigUpdateDetails, LoginAuditDetails,
        MailAuditDetails, TaskRetryAuditDetails, UserAuditDetails, UserSessionRevokeAuditDetails,
        details,
    };
    use crate::types::{
        BackgroundTaskKind, BackgroundTaskStatus, SystemConfigVisibility, UserRole, UserStatus,
    };
    use chrono::Utc;

    #[test]
    fn details_serializes_config_update_and_omits_missing_prior_visibility() {
        assert_eq!(
            details(ConfigUpdateDetails {
                value: "public title",
                visibility: SystemConfigVisibility::Public,
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
                visibility: SystemConfigVisibility::Authenticated,
                prior_visibility: Some(SystemConfigVisibility::Private),
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
