//! Background task domain types: stored JSON payload wrappers, task kinds,
//! and task lifecycle states.

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use std::fmt;
#[cfg(all(debug_assertions, feature = "openapi"))]
use utoipa::ToSchema;

/// DB wrapper for the task's dispatch payload JSON stored in
/// `background_tasks.payload_json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, DeriveValueType)]
pub struct StoredTaskPayload(pub String);

impl AsRef<str> for StoredTaskPayload {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for StoredTaskPayload {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<StoredTaskPayload> for String {
    fn from(value: StoredTaskPayload) -> Self {
        value.0
    }
}

/// DB wrapper for the task's final result JSON stored in
/// `background_tasks.result_json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, DeriveValueType)]
pub struct StoredTaskResult(pub String);

impl AsRef<str> for StoredTaskResult {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for StoredTaskResult {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<StoredTaskResult> for String {
    fn from(value: StoredTaskResult) -> Self {
        value.0
    }
}

/// DB wrapper for the task's in-flight runtime JSON stored in
/// `background_tasks.runtime_json` (checkpoint data written during runs).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, DeriveValueType)]
pub struct StoredTaskRuntime(pub String);

impl AsRef<str> for StoredTaskRuntime {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for StoredTaskRuntime {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<StoredTaskRuntime> for String {
    fn from(value: StoredTaskRuntime) -> Self {
        value.0
    }
}

/// DB wrapper for the task's step list JSON stored in
/// `background_tasks.steps_json`, a serialized `Vec<TaskStepInfo>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, DeriveValueType)]
pub struct StoredTaskSteps(pub String);

impl AsRef<str> for StoredTaskSteps {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for StoredTaskSteps {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<StoredTaskSteps> for String {
    fn from(value: StoredTaskSteps) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(32))")]
/// Identifies which background job a task record runs.
#[serde(rename_all = "snake_case")]
pub enum BackgroundTaskKind {
    /// One of the scheduled system maintenance sub-jobs (mail outbox dispatch,
    /// health check, session/token/audit cleanup).
    #[sea_orm(string_value = "system_runtime")]
    SystemRuntime,
    /// Generates the preview image for an uploaded texture.
    #[sea_orm(string_value = "texture_preview_generate")]
    TexturePreviewGenerate,
}

impl BackgroundTaskKind {
    /// Canonical storage string for this kind.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SystemRuntime => "system_runtime",
            Self::TexturePreviewGenerate => "texture_preview_generate",
        }
    }
}

impl fmt::Display for BackgroundTaskKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[cfg_attr(all(debug_assertions, feature = "openapi"), derive(ToSchema))]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::N(16))")]
/// Dispatch lifecycle state of a background task record.
#[serde(rename_all = "snake_case")]
pub enum BackgroundTaskStatus {
    /// Waiting for the dispatcher to lease it.
    #[sea_orm(string_value = "pending")]
    Pending,
    /// Currently held under a processing lease.
    #[sea_orm(string_value = "processing")]
    Processing,
    /// Waiting for its next attempt after a retryable failure.
    #[sea_orm(string_value = "retry")]
    Retry,
    /// Finished successfully.
    #[sea_orm(string_value = "succeeded")]
    Succeeded,
    /// Exhausted its attempts without success.
    #[sea_orm(string_value = "failed")]
    Failed,
    /// Canceled by an administrator before completion.
    #[sea_orm(string_value = "canceled")]
    Canceled,
}

impl BackgroundTaskStatus {
    /// Canonical storage string for this status.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Processing => "processing",
            Self::Retry => "retry",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Canceled => "canceled",
        }
    }

    /// Whether no further dispatch will happen for this status.
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Canceled)
    }
}

impl fmt::Display for BackgroundTaskStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}
