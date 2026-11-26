//! Pure protocol validation rules.
//!
//! These rules describe the Yggdrasil/authlib-injector wire contract itself:
//! blank-string rejection, unsigned (dash-less) UUID shape, the single valid
//! agent name, and the canonical Minecraft profile name charset. Hosts may
//! layer stricter product policy on top, but cannot relax these.

use validator::ValidationError;

/// Rejects strings that are empty or contain only whitespace.
pub fn validate_non_blank(value: &str) -> Result<(), ValidationError> {
    if value.trim().is_empty() {
        return Err(message_validation_error("value cannot be empty"));
    }
    Ok(())
}

/// Optional-field form of [`validate_non_blank`] for `Option<String>` values
/// that are flattened to strings before validation.
pub fn validate_optional_non_blank(value: &str) -> Result<(), ValidationError> {
    validate_non_blank(value)
}

/// Accepts only unsigned (dash-less) UUIDs: exactly 32 hexadecimal characters,
/// matching the protocol's on-wire profile/account id shape.
pub fn validate_unsigned_uuid(value: &str) -> Result<(), ValidationError> {
    if value.len() == 32 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Ok(());
    }
    Err(message_validation_error(
        "uuid must be a 32-character unsigned hexadecimal UUID",
    ))
}

/// Optional-field form of [`validate_unsigned_uuid`].
pub fn validate_optional_unsigned_uuid(value: &str) -> Result<(), ValidationError> {
    validate_unsigned_uuid(value)
}

/// Accepts only the single protocol agent name `Minecraft` (case-sensitive).
pub fn validate_yggdrasil_agent_name(value: &str) -> Result<(), ValidationError> {
    if value == "Minecraft" {
        return Ok(());
    }
    Err(message_validation_error("agent name must be Minecraft"))
}

/// Whether `name` matches the canonical Minecraft profile name rule:
/// 3-16 ASCII letters, numbers, or underscores.
pub fn minecraft_profile_name_is_valid(name: &str) -> bool {
    (3..=16).contains(&name.len())
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

/// Validator form of [`minecraft_profile_name_is_valid`].
pub fn validate_minecraft_profile_name(value: &str) -> Result<(), ValidationError> {
    if minecraft_profile_name_is_valid(value) {
        return Ok(());
    }
    Err(message_validation_error(
        "profile name must be 3-16 ASCII letters, numbers, or underscores",
    ))
}

/// Optional-field form of [`validate_minecraft_profile_name`].
pub fn validate_optional_minecraft_profile_name(value: &str) -> Result<(), ValidationError> {
    validate_minecraft_profile_name(value)
}

/// Builds a `validator` error carrying `message` as the client-facing text.
pub fn message_validation_error(message: impl Into<String>) -> ValidationError {
    let mut validation_error = ValidationError::new("invalid");
    validation_error.message = Some(std::borrow::Cow::Owned(message.into()));
    validation_error
}

#[cfg(test)]
mod tests {
    use super::{
        minecraft_profile_name_is_valid, validate_minecraft_profile_name, validate_non_blank,
        validate_optional_non_blank, validate_unsigned_uuid, validate_yggdrasil_agent_name,
    };

    #[test]
    fn blank_strings_are_rejected() {
        assert!(validate_non_blank("").is_err());
        assert!(validate_non_blank("   ").is_err());
        assert!(validate_optional_non_blank("a").is_ok());
    }

    #[test]
    fn only_unsigned_32_char_hex_uuids_are_accepted() {
        assert!(validate_unsigned_uuid("d8f78c67e4c64b6cae5c6e21687ad818").is_ok());
        assert!(validate_unsigned_uuid("D8F78C67E4C64B6CAE5C6E21687AD818").is_ok());
        // Dashed form is rejected: the protocol uses unsigned UUIDs on the wire.
        assert!(validate_unsigned_uuid("d8f78c67-e4c6-4b6c-ae5c-6e21687ad818").is_err());
        assert!(validate_unsigned_uuid("d8f78c67e4c64b6cae5c6e21687ad81").is_err());
        assert!(validate_unsigned_uuid("zzf78c67e4c64b6cae5c6e21687ad818").is_err());
    }

    #[test]
    fn agent_name_must_be_minecraft() {
        assert!(validate_yggdrasil_agent_name("Minecraft").is_ok());
        assert!(validate_yggdrasil_agent_name("minecraft").is_err());
        assert!(validate_yggdrasil_agent_name("Scrolls").is_err());
    }

    #[test]
    fn profile_name_rule_matches_minecraft_charset() {
        for valid in ["abc", "Notch", "AptS_1547", "x".repeat(16).as_str()] {
            assert!(
                minecraft_profile_name_is_valid(valid),
                "{valid} should pass"
            );
            assert!(validate_minecraft_profile_name(valid).is_ok());
        }
        for invalid in ["ab", "", "has space", "has-dash", "中文", &"x".repeat(17)] {
            assert!(
                !minecraft_profile_name_is_valid(invalid),
                "{invalid} should fail"
            );
            assert!(validate_minecraft_profile_name(invalid).is_err());
        }
    }
}
