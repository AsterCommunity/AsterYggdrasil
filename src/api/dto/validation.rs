use std::borrow::Cow;

use crate::errors::{AsterError, Result};
use validator::{Validate, ValidationError, ValidationErrors, ValidationErrorsKind};

pub(crate) fn validate_request<T: Validate>(value: &T) -> Result<()> {
    value.validate().map_err(validation_errors_to_aster)
}

pub(crate) fn validate_non_blank(value: &str) -> std::result::Result<(), ValidationError> {
    if value.trim().is_empty() {
        return Err(message_validation_error("value cannot be empty"));
    }
    Ok(())
}

pub(crate) fn validate_optional_non_blank(value: &str) -> std::result::Result<(), ValidationError> {
    validate_non_blank(value)
}

pub(crate) fn validate_unsigned_uuid(value: &str) -> std::result::Result<(), ValidationError> {
    if value.len() == 32 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Ok(());
    }
    Err(message_validation_error(
        "uuid must be a 32-character unsigned hexadecimal UUID",
    ))
}

pub(crate) fn validate_yggdrasil_agent_name(
    value: &str,
) -> std::result::Result<(), ValidationError> {
    if value == "Minecraft" {
        return Ok(());
    }
    Err(message_validation_error("agent name must be Minecraft"))
}

pub(crate) fn validate_auth_username(value: &str) -> std::result::Result<(), ValidationError> {
    crate::services::auth_service::validate_username(value).map_err(aster_to_validation_error)
}

pub(crate) fn validate_optional_auth_username(
    value: &str,
) -> std::result::Result<(), ValidationError> {
    validate_auth_username(value)
}

pub(crate) fn validate_auth_email(value: &str) -> std::result::Result<(), ValidationError> {
    crate::services::auth_service::validate_email(value).map_err(aster_to_validation_error)
}

pub(crate) fn validate_auth_password(value: &str) -> std::result::Result<(), ValidationError> {
    crate::services::auth_service::validate_password(value).map_err(aster_to_validation_error)
}

pub(crate) fn validate_optional_auth_password(
    value: &str,
) -> std::result::Result<(), ValidationError> {
    validate_auth_password(value)
}

pub(crate) fn validate_minecraft_profile_name(
    value: &str,
) -> std::result::Result<(), ValidationError> {
    crate::services::yggdrasil_service::validate_profile_name(value)
        .map_err(aster_to_validation_error)
}

pub(crate) fn validate_optional_minecraft_profile_name(
    value: &str,
) -> std::result::Result<(), ValidationError> {
    validate_minecraft_profile_name(value)
}

pub(crate) fn validate_optional_unsigned_uuid(
    value: &str,
) -> std::result::Result<(), ValidationError> {
    validate_unsigned_uuid(value)
}

fn aster_to_validation_error(error: AsterError) -> ValidationError {
    let mut validation_error = ValidationError::new("invalid");
    validation_error.message = Some(Cow::Owned(error.message().to_string()));
    validation_error
}

pub(crate) fn message_validation_error(message: impl Into<String>) -> ValidationError {
    let mut validation_error = ValidationError::new("invalid");
    validation_error.message = Some(Cow::Owned(message.into()));
    validation_error
}

fn validation_errors_to_aster(errors: ValidationErrors) -> AsterError {
    let mut messages = Vec::new();
    collect_validation_messages(&errors, None, &mut messages);
    messages.sort();
    AsterError::validation_error(messages.join(", "))
}

fn collect_validation_messages(
    errors: &ValidationErrors,
    prefix: Option<&str>,
    messages: &mut Vec<String>,
) {
    for (field, kind) in errors.errors() {
        let field_path = if field == "__all__" {
            prefix.map(str::to_string)
        } else {
            Some(match prefix {
                Some(prefix) => format!("{prefix}.{field}"),
                None => field.to_string(),
            })
        };

        match kind {
            ValidationErrorsKind::Field(field_errors) => {
                for error in field_errors {
                    messages.push(validation_error_message(field_path.as_deref(), error));
                }
            }
            ValidationErrorsKind::Struct(child) => {
                collect_validation_messages(child, field_path.as_deref(), messages);
            }
            ValidationErrorsKind::List(children) => {
                for (index, child) in children {
                    let list_path = match &field_path {
                        Some(path) => format!("{path}[{index}]"),
                        None => format!("[{index}]"),
                    };
                    collect_validation_messages(child, Some(&list_path), messages);
                }
            }
        }
    }
}

fn validation_error_message(field: Option<&str>, error: &ValidationError) -> String {
    error
        .message
        .as_ref()
        .map(ToString::to_string)
        .unwrap_or_else(|| match field {
            Some(field) => format!("invalid field '{field}'"),
            None => "invalid request".to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(validator::Validate)]
    struct NonBlankReq {
        #[validate(custom(function = "crate::api::dto::validation::validate_non_blank"))]
        name: String,
    }

    #[derive(validator::Validate)]
    struct SizeReq {
        #[validate(range(min = 1, message = "page_size must be positive"))]
        page_size: i64,
    }

    #[test]
    fn validate_request_uses_custom_messages() {
        let err = validate_request(&NonBlankReq {
            name: " ".to_string(),
        })
        .unwrap_err();
        assert_eq!(err.message(), "value cannot be empty");
    }

    #[test]
    fn validate_request_surfaces_range_messages() {
        let err = validate_request(&SizeReq { page_size: 0 }).unwrap_err();
        assert_eq!(err.message(), "page_size must be positive");
    }

    #[derive(validator::Validate)]
    #[validate(schema(function = "validate_same_values", skip_on_field_errors = false))]
    struct NestedReq {
        #[validate(nested)]
        items: Vec<NonBlankReq>,
    }

    fn validate_same_values(_value: &NestedReq) -> std::result::Result<(), ValidationError> {
        Err(message_validation_error("items must be unique"))
    }

    #[test]
    fn validate_request_collects_nested_and_schema_messages() {
        let err = validate_request(&NestedReq {
            items: vec![NonBlankReq {
                name: String::new(),
            }],
        })
        .unwrap_err();
        assert_eq!(err.message(), "items must be unique, value cannot be empty");
    }
}
