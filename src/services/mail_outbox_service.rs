//! Mail outbox dispatch service.

use std::sync::Arc;

use chrono::{Duration, Utc};
use sea_orm::{ConnectionTrait, DatabaseConnection, Set};

use crate::config::RuntimeConfig;
use crate::db::repository::mail_outbox_repo;
use crate::entities::mail_outbox;
use crate::errors::Result;
use crate::runtime::MailRuntimeState;
use crate::services::{
    mail_audit_service, mail_service,
    mail_service::MailSender,
    mail_template::{self, MailTemplatePayload},
};
use crate::types::MailOutboxStatus;
use aster_forge_mail::MailOutboxDeliveryFailureDecision;

const MAIL_OUTBOX_BATCH_SIZE: u64 = 20;
const MAIL_OUTBOX_PROCESSING_STALE_SECS: i64 = 60;
const MAIL_OUTBOX_MAX_ATTEMPTS: i32 = 6;
const MAIL_OUTBOX_DRAIN_MAX_ROUNDS: usize = 32;
const MAIL_OUTBOX_RETRY_POLICY: aster_forge_mail::MailOutboxRetryPolicy =
    aster_forge_mail::MailOutboxRetryPolicy::new(
        MAIL_OUTBOX_MAX_ATTEMPTS,
        aster_forge_mail::DEFAULT_ERROR_MAX_LEN,
    );

pub use aster_forge_mail::DispatchStats;

pub async fn enqueue<C: ConnectionTrait>(
    db: &C,
    to_address: &str,
    to_name: Option<&str>,
    payload: MailTemplatePayload,
) -> Result<mail_outbox::Model> {
    let now = Utc::now();
    let template_code = payload.template_code();
    tracing::debug!(
        template_code = %template_code.as_str(),
        to_address = to_address,
        has_to_name = to_name.is_some(),
        "enqueueing mail outbox row"
    );
    mail_outbox_repo::create(
        db,
        mail_outbox::ActiveModel {
            template_code: Set(template_code),
            to_address: Set(to_address.to_string()),
            to_name: Set(to_name.map(str::to_string)),
            payload_json: Set(payload.to_stored()?),
            status: Set(MailOutboxStatus::Pending),
            attempt_count: Set(0),
            next_attempt_at: Set(now),
            processing_started_at: Set(None),
            sent_at: Set(None),
            last_error: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        },
    )
    .await
    .inspect(|row| {
        tracing::debug!(
            mail_outbox_id = row.id,
            template_code = %row.template_code.as_str(),
            "enqueued mail outbox row"
        );
    })
}

pub async fn dispatch_due(state: &impl MailRuntimeState) -> Result<DispatchStats> {
    dispatch_due_with(
        state.writer_db(),
        state.runtime_config(),
        state.mail_sender(),
    )
    .await
}

pub async fn dispatch_due_with(
    db: &DatabaseConnection,
    runtime_config: &Arc<RuntimeConfig>,
    mail_sender: &Arc<dyn MailSender>,
) -> Result<DispatchStats> {
    let now = Utc::now();
    let stale_before = now - Duration::seconds(MAIL_OUTBOX_PROCESSING_STALE_SECS);
    let due =
        mail_outbox_repo::list_claimable(db, now, stale_before, MAIL_OUTBOX_BATCH_SIZE).await?;
    let mut stats = DispatchStats::default();
    tracing::debug!(
        batch_size = MAIL_OUTBOX_BATCH_SIZE,
        due_count = due.len(),
        stale_before = %stale_before,
        "dispatching due mail outbox rows"
    );

    for row in due {
        let claimed_at = Utc::now();
        if !mail_outbox_repo::try_claim(db, row.id, claimed_at, stale_before).await? {
            tracing::debug!(
                mail_outbox_id = row.id,
                template_code = %row.template_code.as_str(),
                "mail outbox claim skipped because row was already claimed"
            );
            continue;
        }

        stats.claimed += 1;
        tracing::debug!(
            mail_outbox_id = row.id,
            template_code = %row.template_code.as_str(),
            attempt_count = row.attempt_count,
            "claimed mail outbox row"
        );
        let mut claimed_row = row;
        claimed_row.status = MailOutboxStatus::Processing;
        claimed_row.processing_started_at = Some(claimed_at);
        claimed_row.updated_at = claimed_at;

        match deliver_one(runtime_config, mail_sender, &claimed_row).await {
            Ok(subject) => {
                tracing::debug!(
                    mail_outbox_id = claimed_row.id,
                    template_code = %claimed_row.template_code.as_str(),
                    "mail outbox SMTP delivery succeeded"
                );
                // SMTP succeeded, so the outbox row must be marked as sent with best effort.
                // Otherwise it can remain `Processing`, be reclaimed as stale, and produce a
                // duplicate delivery. The short retry window keeps transient database failures from
                // becoming duplicate mail in normal operation.
                match mark_sent_with_retry(db, claimed_row.id).await {
                    Ok(true) => {
                        stats.sent += 1;
                        mail_audit_service::log_send_with_db(
                            db,
                            runtime_config,
                            mail_audit_service::MailAuditInput {
                                actor_user_id: 0,
                                ip_address: None,
                                user_agent: None,
                                to_address: &claimed_row.to_address,
                                to_name: claimed_row.to_name.as_deref(),
                                template_code: claimed_row.template_code.as_str(),
                                subject: Some(&subject),
                                outbox_id: Some(claimed_row.id),
                                attempt_count: Some(claimed_row.attempt_count + 1),
                                error: None,
                            },
                        )
                        .await;
                    }
                    Ok(false) => {
                        tracing::warn!(
                            mail_outbox_id = claimed_row.id,
                            template_code = %claimed_row.template_code.as_str(),
                            to = %claimed_row.to_address,
                            "mark_sent affected 0 rows after successful delivery; state will be rechecked"
                        );
                        tracing::debug!(
                            mail_outbox_id = claimed_row.id,
                            "mail outbox mark_sent returned 0 rows after delivery"
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            mail_outbox_id = claimed_row.id,
                            template_code = %claimed_row.template_code.as_str(),
                            to = %claimed_row.to_address,
                            stale_secs = MAIL_OUTBOX_PROCESSING_STALE_SECS,
                            error = %e,
                            "CRITICAL: SMTP delivery succeeded but mark_sent failed after all retries; \
                             row remains Processing and may be re-claimed, causing duplicate delivery"
                        );
                        tracing::debug!(
                            mail_outbox_id = claimed_row.id,
                            error = %e,
                            "mail outbox mark_sent exhausted retries"
                        );
                    }
                }
            }
            Err(error) => {
                let attempt_count = claimed_row.attempt_count + 1;
                match MAIL_OUTBOX_RETRY_POLICY
                    .delivery_failure_decision(attempt_count, error.to_string())
                {
                    MailOutboxDeliveryFailureDecision::PermanentFailure {
                        attempt_count,
                        error_message,
                    } => {
                        if mail_outbox_repo::mark_failed(
                            db,
                            claimed_row.id,
                            attempt_count,
                            Utc::now(),
                            &error_message,
                        )
                        .await?
                        {
                            stats.failed += 1;
                            mail_audit_service::log_delivery_failed_with_db(
                                db,
                                runtime_config,
                                mail_audit_service::MailAuditInput {
                                    actor_user_id: 0,
                                    ip_address: None,
                                    user_agent: None,
                                    to_address: &claimed_row.to_address,
                                    to_name: claimed_row.to_name.as_deref(),
                                    template_code: claimed_row.template_code.as_str(),
                                    subject: None,
                                    outbox_id: Some(claimed_row.id),
                                    attempt_count: Some(attempt_count),
                                    error: Some(&error_message),
                                },
                            )
                            .await;
                        }
                        tracing::debug!(
                            mail_outbox_id = claimed_row.id,
                            attempt_count,
                            "mail outbox delivery permanently failed"
                        );
                        tracing::warn!(
                            mail_outbox_id = claimed_row.id,
                            template_code = %claimed_row.template_code.as_str(),
                            to = %claimed_row.to_address,
                            attempt_count,
                            error = %error_message,
                            "mail outbox delivery permanently failed"
                        );
                    }
                    MailOutboxDeliveryFailureDecision::Retry {
                        attempt_count,
                        retry_delay_secs,
                        error_message,
                    } => {
                        let retry_at = Utc::now() + Duration::seconds(retry_delay_secs);
                        if mail_outbox_repo::mark_retry(
                            db,
                            claimed_row.id,
                            attempt_count,
                            retry_at,
                            &error_message,
                        )
                        .await?
                        {
                            stats.retried += 1;
                        }
                        tracing::debug!(
                            mail_outbox_id = claimed_row.id,
                            attempt_count,
                            retry_at = %retry_at,
                            "mail outbox delivery scheduled for retry"
                        );
                        tracing::warn!(
                            mail_outbox_id = claimed_row.id,
                            template_code = %claimed_row.template_code.as_str(),
                            to = %claimed_row.to_address,
                            attempt_count,
                            retry_at = %retry_at,
                            error = %error_message,
                            "mail outbox delivery failed; scheduled retry"
                        );
                    }
                }
            }
        }
    }

    tracing::debug!(
        claimed = stats.claimed,
        sent = stats.sent,
        retried = stats.retried,
        failed = stats.failed,
        "finished dispatching due mail outbox rows"
    );
    Ok(stats)
}

pub async fn drain(state: &impl MailRuntimeState) -> Result<DispatchStats> {
    drain_with(
        state.writer_db(),
        state.runtime_config(),
        state.mail_sender(),
    )
    .await
}

pub async fn drain_with(
    db: &DatabaseConnection,
    runtime_config: &Arc<RuntimeConfig>,
    mail_sender: &Arc<dyn MailSender>,
) -> Result<DispatchStats> {
    let mut total = DispatchStats::default();
    tracing::debug!("draining mail outbox");

    for _ in 0..MAIL_OUTBOX_DRAIN_MAX_ROUNDS {
        let stats = dispatch_due_with(db, runtime_config, mail_sender).await?;
        let claimed = stats.claimed;
        total.merge(stats);
        if claimed == 0 {
            tracing::debug!("mail outbox drain finished because no rows were claimed");
            break;
        }
    }

    tracing::debug!(
        claimed = total.claimed,
        sent = total.sent,
        retried = total.retried,
        failed = total.failed,
        "mail outbox drain completed"
    );
    Ok(total)
}

async fn deliver_one(
    runtime_config: &RuntimeConfig,
    mail_sender: &Arc<dyn MailSender>,
    row: &mail_outbox::Model,
) -> Result<String> {
    let rendered = mail_template::render(runtime_config, row.template_code, &row.payload_json)?;
    let subject = rendered.subject.clone();
    tracing::debug!(
        mail_outbox_id = row.id,
        template_code = %row.template_code.as_str(),
        "delivering one mail outbox row"
    );
    mail_service::send_rendered_with(
        runtime_config,
        mail_sender,
        aster_forge_mail::MailRecipient {
            address: row.to_address.clone(),
            display_name: row.to_name.clone(),
        },
        rendered,
    )
    .await?;
    tracing::debug!(
        mail_outbox_id = row.id,
        template_code = %row.template_code.as_str(),
        "delivered one mail outbox row"
    );
    Ok(subject)
}

/// Persists the `sent` state after SMTP success, retrying transient database failures.
///
/// Returns `Ok(true)` when the processing row was marked as sent, `Ok(false)` when no row was
/// updated, and `Err(...)` when all retry attempts failed.
async fn mark_sent_with_retry(db: &DatabaseConnection, id: i64) -> Result<bool> {
    tracing::debug!(mail_outbox_id = id, "marking mail outbox row as sent");
    let result = aster_forge_mail::retry_mark_sent(
        id,
        aster_forge_mail::DEFAULT_MARK_SENT_RETRY_DELAYS_MS,
        |id, _attempt| async move { mail_outbox_repo::mark_sent(db, id, Utc::now()).await },
    )
    .await;
    if result.is_err() {
        tracing::debug!(
            mail_outbox_id = id,
            "mail outbox mark_sent retries exhausted"
        );
    }
    result
}
