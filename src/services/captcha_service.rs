//! Visual captcha challenge service.

use crate::api::error_code::AsterErrorCode;
use crate::cache::CacheExt;
use crate::config::auth_runtime::RuntimeCaptchaPolicy;
use crate::errors::{AsterError, MapAsterErr, Result};
use crate::runtime::{CacheRuntimeState, RuntimeConfigRuntimeState};
use crate::utils::{
    hash, id,
    numbers::{u32_to_usize, u64_to_usize},
};
use captcha_rs::CaptchaBuilder;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

const CACHE_KEY_PREFIX: &str = "auth:captcha:";
const CAPTCHA_CHARSET: &[char] = &[
    '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'C', 'D', 'E', 'F', 'G', 'H', 'J', 'K', 'M', 'N',
    'P', 'Q', 'R', 'T', 'W', 'X', 'Y',
];

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
pub struct CaptchaChallengeResponse {
    pub challenge_id: String,
    pub image_base64: String,
    pub mime: String,
    pub expires_in: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct CaptchaChallengeState {
    answer_hash: String,
    attempts: u64,
    max_attempts: u64,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptchaRequirement {
    Login,
    Register,
    InvitationAccept,
    RegisterActivationResend,
}

impl CaptchaRequirement {
    fn is_required(self, policy: &RuntimeCaptchaPolicy) -> bool {
        match self {
            Self::Login => policy.login_required(),
            Self::Register => policy.register_required(),
            Self::InvitationAccept => policy.invitation_accept_required(),
            Self::RegisterActivationResend => policy.register_activation_resend_required(),
        }
    }
}

struct RenderedCaptchaChallenge {
    answer: String,
    image_base64: String,
}

pub fn policy(state: &impl RuntimeConfigRuntimeState) -> RuntimeCaptchaPolicy {
    RuntimeCaptchaPolicy::from_runtime_config(state.runtime_config())
}

pub fn preview_image(policy: &RuntimeCaptchaPolicy) -> Result<String> {
    Ok(render_challenge(policy)?.image_base64)
}

pub async fn issue_challenge<S>(state: &S) -> Result<CaptchaChallengeResponse>
where
    S: CacheRuntimeState + RuntimeConfigRuntimeState,
{
    let policy = policy(state);
    let captcha = render_challenge(&policy)?;
    let challenge_id = id::new_short_token();
    let state_value = CaptchaChallengeState {
        answer_hash: answer_hash(&captcha.answer),
        attempts: 0,
        max_attempts: policy.max_attempts,
        expires_at: Utc::now()
            + Duration::seconds(
                i64::try_from(policy.ttl_secs)
                    .map_aster_err_ctx("captcha ttl is too large", AsterError::internal_error)?,
            ),
    };

    state
        .cache()
        .set(
            &cache_key(&challenge_id),
            &state_value,
            Some(policy.ttl_secs),
        )
        .await;

    Ok(CaptchaChallengeResponse {
        challenge_id,
        image_base64: captcha.image_base64,
        mime: "image/jpeg".to_string(),
        expires_in: policy.ttl_secs,
    })
}

fn render_challenge(policy: &RuntimeCaptchaPolicy) -> Result<RenderedCaptchaChallenge> {
    let length = u64_to_usize(policy.length, "captcha length")?;
    let render = policy.preset.render_params();
    let compression = u8::try_from(render.compression).map_aster_err_with(|| {
        AsterError::internal_error(format!(
            "captcha compression exceeds u8 range: {}",
            render.compression
        ))
    })?;
    let interference_lines = u32_to_usize(render.interference_lines, "captcha interference lines")?;
    let interference_ellipses = u32_to_usize(
        render.interference_ellipses,
        "captcha interference ellipses",
    )?;
    let captcha = CaptchaBuilder::new()
        .length(length)
        .chars(CAPTCHA_CHARSET.to_vec())
        .width(180)
        .height(render.height)
        .complexity(render.complexity)
        .compression(compression)
        .interference_lines(interference_lines)
        .interference_ellipses(interference_ellipses)
        .distortion(render.distortion)
        .build();
    let image_base64 = captcha.to_base64();
    Ok(RenderedCaptchaChallenge {
        answer: captcha.text,
        image_base64,
    })
}

pub async fn verify_if_required<S>(
    state: &S,
    requirement: CaptchaRequirement,
    challenge_id: Option<&str>,
    answer: Option<&str>,
) -> Result<()>
where
    S: CacheRuntimeState + RuntimeConfigRuntimeState,
{
    if !requirement.is_required(&policy(state)) {
        return Ok(());
    }
    let challenge_id = challenge_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(captcha_required_error)?;
    let answer = answer
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(captcha_required_error)?;
    verify_answer(state, challenge_id, answer).await
}

async fn verify_answer<S>(state: &S, challenge_id: &str, answer: &str) -> Result<()>
where
    S: CacheRuntimeState,
{
    let key = cache_key(challenge_id);
    let Some(mut challenge) = state.cache().get::<CaptchaChallengeState>(&key).await else {
        return Err(captcha_expired_error());
    };
    if challenge.expires_at <= Utc::now() {
        state.cache().delete(&key).await;
        return Err(captcha_expired_error());
    }

    if challenge.answer_hash == answer_hash(answer) {
        state.cache().delete(&key).await;
        return Ok(());
    }

    challenge.attempts = challenge.attempts.saturating_add(1);
    if challenge.attempts >= challenge.max_attempts {
        state.cache().delete(&key).await;
    } else {
        state
            .cache()
            .set(&key, &challenge, Some(remaining_ttl_secs(&challenge)))
            .await;
    }

    Err(AsterError::validation_error_code(
        AsterErrorCode::AuthCaptchaInvalid,
        "captcha answer is invalid",
    ))
}

fn remaining_ttl_secs(challenge: &CaptchaChallengeState) -> u64 {
    (challenge.expires_at - Utc::now()).num_seconds().max(1) as u64
}

fn cache_key(challenge_id: &str) -> String {
    format!("{CACHE_KEY_PREFIX}{challenge_id}")
}

fn answer_hash(answer: &str) -> String {
    hash::sha256_hex(normalize_answer(answer).as_bytes())
}

fn normalize_answer(answer: &str) -> String {
    answer
        .trim()
        .chars()
        .filter(|value| !value.is_whitespace())
        .collect::<String>()
        .to_ascii_uppercase()
}

fn captcha_required_error() -> AsterError {
    AsterError::validation_error_code(
        AsterErrorCode::AuthCaptchaRequired,
        "captcha verification is required",
    )
}

fn captcha_expired_error() -> AsterError {
    AsterError::validation_error_code(
        AsterErrorCode::AuthCaptchaExpired,
        "captcha challenge is expired",
    )
}
