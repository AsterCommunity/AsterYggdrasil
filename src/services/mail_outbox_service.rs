//! Mail outbox dispatch service.

pub mod runtime;

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
    mail_template::{self, MailTemplatePayload},
};
use crate::types::MailOutboxStatus;
use aster_forge_mail::{
    DispatchStats, MailOutboxDispatchConfig, MailOutboxDispatchRow, MailOutboxRetryPolicy,
    MailSender,
};

const MAIL_OUTBOX_BATCH_SIZE: u64 = 20;
const MAIL_OUTBOX_PROCESSING_STALE_SECS: i64 = 60;
const MAIL_OUTBOX_MAX_ATTEMPTS: i32 = 6;
const MAIL_OUTBOX_DRAIN_MAX_ROUNDS: usize = 32;
const MAIL_OUTBOX_DISPATCH_CONFIG: MailOutboxDispatchConfig = MailOutboxDispatchConfig::new(
    MAIL_OUTBOX_BATCH_SIZE,
    MAIL_OUTBOX_PROCESSING_STALE_SECS,
    MAIL_OUTBOX_DRAIN_MAX_ROUNDS,
    MailOutboxRetryPolicy::new(
        MAIL_OUTBOX_MAX_ATTEMPTS,
        aster_forge_mail::DEFAULT_ERROR_MAX_LEN,
    ),
);

impl MailOutboxDispatchRow for mail_outbox::Model {
    fn id(&self) -> i64 {
        self.id
    }

    fn attempt_count(&self) -> i32 {
        self.attempt_count
    }

    fn template_code(&self) -> &str {
        self.template_code.as_str()
    }

    fn to_address(&self) -> &str {
        &self.to_address
    }
}

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
    aster_forge_mail::dispatch_mail_outbox(
        &MAIL_OUTBOX_DISPATCH_CONFIG,
        |batch_size, stale_secs| async move {
            let now = Utc::now();
            let stale_before = now - Duration::seconds(stale_secs);
            mail_outbox_repo::list_claimable(db, now, stale_before, batch_size).await
        },
        |row| async move {
            let now = Utc::now();
            let stale_before = now - Duration::seconds(MAIL_OUTBOX_PROCESSING_STALE_SECS);
            mail_outbox_repo::try_claim(db, row.id, now, stale_before).await
        },
        |row| async move { deliver_one(runtime_config, mail_sender, &row).await },
        |id, _attempt| async move { mail_outbox_repo::mark_sent(db, id, Utc::now()).await },
        |row, attempt_count, retry_delay_secs, error_message| async move {
            let retry_at = Utc::now() + Duration::seconds(retry_delay_secs);
            mail_outbox_repo::mark_retry(db, row.id, attempt_count, retry_at, &error_message).await
        },
        |row, attempt_count, error_message| async move {
            mail_outbox_repo::mark_failed(db, row.id, attempt_count, Utc::now(), &error_message)
                .await
        },
        |row, attempt_count, subject| async move {
            mail_audit_service::log_send_with_db(
                db,
                runtime_config,
                mail_audit_service::MailAuditInput {
                    actor_user_id: 0,
                    ip_address: None,
                    user_agent: None,
                    to_address: &row.to_address,
                    to_name: row.to_name.as_deref(),
                    template_code: row.template_code.as_str(),
                    subject: Some(&subject),
                    outbox_id: Some(row.id),
                    attempt_count: Some(attempt_count),
                    error: None,
                },
            )
            .await;
        },
        |row, attempt_count, error_message| async move {
            mail_audit_service::log_delivery_failed_with_db(
                db,
                runtime_config,
                mail_audit_service::MailAuditInput {
                    actor_user_id: 0,
                    ip_address: None,
                    user_agent: None,
                    to_address: &row.to_address,
                    to_name: row.to_name.as_deref(),
                    template_code: row.template_code.as_str(),
                    subject: None,
                    outbox_id: Some(row.id),
                    attempt_count: Some(attempt_count),
                    error: Some(&error_message),
                },
            )
            .await;
        },
    )
    .await
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
    aster_forge_mail::drain_mail_outbox(&MAIL_OUTBOX_DISPATCH_CONFIG, || async move {
        dispatch_due_with(db, runtime_config, mail_sender).await
    })
    .await
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
