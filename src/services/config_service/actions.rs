use std::collections::BTreeMap;

use crate::config::yggdrasil::{
    YGGDRASIL_SIGNATURE_PRIVATE_KEY_KEY, YGGDRASIL_SIGNATURE_PUBLIC_KEY_KEY,
};
use crate::config::{auth_runtime, mail, system_config as runtime_system_config};
use crate::db::repository::{system_config_repo, user_repo};
use crate::errors::{AsterError, Result};
use crate::runtime::MailRuntimeState;
use crate::services::{
    audit_service::{self, AuditContext},
    captcha_service, mail_audit_service, mail_service, yggdrasil_signature,
};
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

use super::SystemConfigValue;

pub const MAIL_CONFIG_ACTION_KEY: &str = "mail";
pub const AUTH_CAPTCHA_CONFIG_ACTION_KEY: &str = "auth_captcha";
pub const YGGDRASIL_CONFIG_ACTION_KEY: &str = "yggdrasil";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub enum ConfigActionType {
    SendTestEmail,
    PreviewCaptcha,
    RotateYggdrasilSignatureKey,
}

impl ConfigActionType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SendTestEmail => "send_test_email",
            Self::PreviewCaptcha => "preview_captcha",
            Self::RotateYggdrasilSignatureKey => "rotate_yggdrasil_signature_key",
        }
    }

    const fn should_audit(self) -> bool {
        match self {
            Self::SendTestEmail | Self::RotateYggdrasilSignatureKey => true,
            Self::PreviewCaptcha => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigActionResult {
    pub message: String,
    pub target_email: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct ExecuteConfigActionInput<'a> {
    pub key: &'a str,
    pub action: ConfigActionType,
    pub actor_user_id: i64,
    pub values: Option<&'a BTreeMap<String, SystemConfigValue>>,
}

pub async fn execute_action_with_audit(
    state: &impl MailRuntimeState,
    input: ExecuteConfigActionInput<'_>,
    audit_ctx: &AuditContext,
) -> Result<ConfigActionResult> {
    tracing::debug!(
        key = input.key,
        action = %input.action.as_str(),
        actor_user_id = input.actor_user_id,
        has_values = input.values.is_some_and(|values| !values.is_empty()),
        "executing config action"
    );
    let action_result = match input.key {
        MAIL_CONFIG_ACTION_KEY => execute_mail_action(state, input, audit_ctx).await,
        AUTH_CAPTCHA_CONFIG_ACTION_KEY => execute_auth_captcha_action(state, input).await,
        YGGDRASIL_CONFIG_ACTION_KEY => execute_yggdrasil_action(state, input).await,
        key => Err(AsterError::record_not_found(format!(
            "config action target '{key}'"
        ))),
    }?;
    tracing::debug!(
        key = input.key,
        action = %input.action.as_str(),
        target_email = action_result.target_email.as_deref().unwrap_or(""),
        "config action executed"
    );
    if input.action.should_audit() {
        audit_service::log_with_details(
            state,
            audit_ctx,
            audit_service::AuditAction::ConfigActionExecute,
            audit_service::AuditEntityType::SystemConfig,
            None,
            Some(input.key),
            || {
                audit_service::details(audit_service::ConfigActionDetails {
                    action: input.action.as_str(),
                    target_email: action_result.target_email.as_deref(),
                })
            },
        )
        .await;
    }
    Ok(action_result)
}

async fn execute_mail_action(
    state: &impl MailRuntimeState,
    input: ExecuteConfigActionInput<'_>,
    audit_ctx: &AuditContext,
) -> Result<ConfigActionResult> {
    match input.action {
        ConfigActionType::SendTestEmail => {
            let actor = user_repo::find_by_id(state.reader_db(), input.actor_user_id).await?;
            let requested_target = mail_action_target_email(input.values)?;
            let requested_target = requested_target.as_deref().unwrap_or(&actor.email);
            let normalized_target = mail::normalize_mail_address_config_value(requested_target)?;
            if normalized_target.is_empty() {
                return Err(AsterError::validation_error("target_email is required"));
            }

            tracing::debug!(
                actor_user_id = input.actor_user_id,
                actor_username = %actor.username,
                target_email = %normalized_target,
                action = %input.action.as_str(),
                "config: executing mail action"
            );

            let result =
                mail_service::send_test_email(state, &normalized_target, Some(&actor.username))
                    .await;
            let ip_address = audit_ctx.ip_address.as_deref();
            let user_agent = audit_ctx.user_agent.as_deref();
            match &result {
                Ok(()) => {
                    mail_audit_service::log_send(
                        state,
                        mail_audit_service::MailAuditInput {
                            actor_user_id: input.actor_user_id,
                            ip_address,
                            user_agent,
                            to_address: &normalized_target,
                            to_name: None,
                            template_code: "smtp_test",
                            subject: Some("AsterYggdrasil SMTP test"),
                            outbox_id: None,
                            attempt_count: None,
                            error: None,
                        },
                    )
                    .await;
                }
                Err(error) => {
                    let error_message = error.to_string();
                    mail_audit_service::log_delivery_failed_with_db(
                        state.writer_db(),
                        state.runtime_config(),
                        mail_audit_service::MailAuditInput {
                            actor_user_id: input.actor_user_id,
                            ip_address,
                            user_agent,
                            to_address: &normalized_target,
                            to_name: None,
                            template_code: "smtp_test",
                            subject: Some("AsterYggdrasil SMTP test"),
                            outbox_id: None,
                            attempt_count: None,
                            error: Some(&error_message),
                        },
                    )
                    .await;
                }
            }
            result?;

            tracing::debug!(
                actor_user_id = input.actor_user_id,
                target_email = %normalized_target,
                "config mail test email completed"
            );
            Ok(ConfigActionResult {
                message: format!("Test email sent to {normalized_target}"),
                target_email: Some(normalized_target),
                value: None,
            })
        }
        ConfigActionType::RotateYggdrasilSignatureKey => Err(AsterError::validation_error(
            "rotate_yggdrasil_signature_key is only supported by yggdrasil config actions",
        )),
        ConfigActionType::PreviewCaptcha => Err(AsterError::validation_error(
            "preview_captcha is only supported by auth captcha config actions",
        )),
    }
}

fn mail_action_target_email(
    values: Option<&BTreeMap<String, SystemConfigValue>>,
) -> Result<Option<String>> {
    let Some(values) = values else {
        return Ok(None);
    };
    let mut target_email = None;
    for (key, value) in values {
        if key != "target_email" {
            return Err(AsterError::validation_error(format!(
                "{key} is not supported by mail config actions"
            )));
        }
        let SystemConfigValue::String(value) = value else {
            return Err(AsterError::validation_error(
                "target_email action value must be a string",
            ));
        };
        let value = value.trim();
        if !value.is_empty() {
            target_email = Some(value.to_string());
        }
    }
    Ok(target_email)
}

async fn execute_auth_captcha_action(
    state: &impl MailRuntimeState,
    input: ExecuteConfigActionInput<'_>,
) -> Result<ConfigActionResult> {
    match input.action {
        ConfigActionType::PreviewCaptcha => {
            let overrides = normalize_captcha_action_values(state, input.values)?;
            let policy = auth_runtime::RuntimeCaptchaPolicy::from_runtime_config_with_overrides(
                state.runtime_config(),
                &overrides,
            );
            Ok(ConfigActionResult {
                message: "Captcha preview generated".to_string(),
                target_email: None,
                value: Some(captcha_service::preview_image(&policy)?),
            })
        }
        ConfigActionType::SendTestEmail => Err(AsterError::validation_error(
            "send_test_email is only supported by mail config actions",
        )),
        ConfigActionType::RotateYggdrasilSignatureKey => Err(AsterError::validation_error(
            "rotate_yggdrasil_signature_key is only supported by yggdrasil config actions",
        )),
    }
}

async fn execute_yggdrasil_action(
    state: &impl MailRuntimeState,
    input: ExecuteConfigActionInput<'_>,
) -> Result<ConfigActionResult> {
    match input.action {
        ConfigActionType::RotateYggdrasilSignatureKey => {
            tracing::debug!(
                actor_user_id = input.actor_user_id,
                "rotating yggdrasil signature key"
            );
            let private_key = yggdrasil_signature::generate_private_key_pem(4096)?;
            let public_key =
                yggdrasil_signature::public_key_pem_from_private_key_pem(&private_key)?;
            let private_saved = system_config_repo::upsert_with_options(
                state.writer_db(),
                YGGDRASIL_SIGNATURE_PRIVATE_KEY_KEY,
                &private_key,
                None,
                Some(input.actor_user_id),
            )
            .await?;
            let public_saved = system_config_repo::upsert_with_options(
                state.writer_db(),
                YGGDRASIL_SIGNATURE_PUBLIC_KEY_KEY,
                &public_key,
                None,
                Some(input.actor_user_id),
            )
            .await?;
            state.runtime_config().apply(private_saved);
            state.runtime_config().apply(public_saved);
            tracing::debug!(
                actor_user_id = input.actor_user_id,
                "yggdrasil signature key rotated"
            );

            Ok(ConfigActionResult {
                message: "Yggdrasil signature key rotated; new profile and hasJoined texture properties will be signed with the new key".to_string(),
                target_email: None,
                value: None,
            })
        }
        ConfigActionType::SendTestEmail => Err(AsterError::validation_error(
            "send_test_email is only supported by mail config actions",
        )),
        ConfigActionType::PreviewCaptcha => Err(AsterError::validation_error(
            "preview_captcha is only supported by auth captcha config actions",
        )),
    }
}

fn normalize_captcha_action_values(
    state: &impl MailRuntimeState,
    values: Option<&BTreeMap<String, SystemConfigValue>>,
) -> Result<BTreeMap<String, String>> {
    let mut overrides = BTreeMap::new();
    let Some(values) = values else {
        return Ok(overrides);
    };

    for (key, value) in values {
        if !is_auth_captcha_config_key(key) {
            return Err(AsterError::validation_error(format!(
                "{key} is not supported by captcha preview"
            )));
        }
        let Some(definition) = runtime_system_config::get_definition(key) else {
            return Err(AsterError::validation_error(format!(
                "unknown captcha config key: {key}"
            )));
        };
        let storage_value = value.to_storage_for_type(definition.value_type)?;
        runtime_system_config::validate_value_type(definition.value_type, &storage_value)?;
        let normalized_value = runtime_system_config::normalize_system_value(
            state.runtime_config(),
            key,
            &storage_value,
        )?;
        overrides.insert(key.clone(), normalized_value);
    }

    Ok(overrides)
}

fn is_auth_captcha_config_key(key: &str) -> bool {
    matches!(
        key,
        auth_runtime::AUTH_CAPTCHA_ENABLED_KEY
            | auth_runtime::AUTH_CAPTCHA_LOGIN_REQUIRED_KEY
            | auth_runtime::AUTH_CAPTCHA_REGISTER_REQUIRED_KEY
            | auth_runtime::AUTH_CAPTCHA_INVITATION_ACCEPT_REQUIRED_KEY
            | auth_runtime::AUTH_CAPTCHA_REGISTER_ACTIVATION_RESEND_REQUIRED_KEY
            | auth_runtime::AUTH_CAPTCHA_TTL_SECS_KEY
            | auth_runtime::AUTH_CAPTCHA_PRESET_KEY
            | auth_runtime::AUTH_CAPTCHA_LENGTH_KEY
            | auth_runtime::AUTH_CAPTCHA_MAX_ATTEMPTS_KEY
    )
}

#[cfg(test)]
mod tests {
    use super::{
        AUTH_CAPTCHA_CONFIG_ACTION_KEY, ConfigActionType, MAIL_CONFIG_ACTION_KEY,
        SystemConfigValue, YGGDRASIL_CONFIG_ACTION_KEY, mail_action_target_email,
    };
    use std::collections::BTreeMap;

    #[test]
    fn config_action_type_exposes_stable_wire_value() {
        assert_eq!(ConfigActionType::SendTestEmail.as_str(), "send_test_email");
        assert_eq!(ConfigActionType::PreviewCaptcha.as_str(), "preview_captcha");
        assert_eq!(
            ConfigActionType::RotateYggdrasilSignatureKey.as_str(),
            "rotate_yggdrasil_signature_key"
        );
        assert_eq!(MAIL_CONFIG_ACTION_KEY, "mail");
        assert_eq!(AUTH_CAPTCHA_CONFIG_ACTION_KEY, "auth_captcha");
        assert_eq!(YGGDRASIL_CONFIG_ACTION_KEY, "yggdrasil");
    }

    #[test]
    fn mail_action_target_email_reads_values() {
        let mut values = BTreeMap::new();
        values.insert(
            "target_email".to_string(),
            SystemConfigValue::String(" ops@example.com ".to_string()),
        );

        assert_eq!(
            mail_action_target_email(Some(&values)).unwrap().as_deref(),
            Some("ops@example.com")
        );

        values.insert(
            "target_email".to_string(),
            SystemConfigValue::String("   ".to_string()),
        );
        assert_eq!(mail_action_target_email(Some(&values)).unwrap(), None);
    }

    #[test]
    fn mail_action_target_email_rejects_invalid_values() {
        let mut unknown = BTreeMap::new();
        unknown.insert(
            "unexpected".to_string(),
            SystemConfigValue::String("value".to_string()),
        );
        assert!(mail_action_target_email(Some(&unknown)).is_err());

        let mut invalid = BTreeMap::new();
        invalid.insert(
            "target_email".to_string(),
            SystemConfigValue::StringArray(vec!["ops@example.com".to_string()]),
        );
        assert!(mail_action_target_email(Some(&invalid)).is_err());
    }
}
