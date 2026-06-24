//! Background task step helpers.

use crate::errors::{AsterError, MapAsterErr, Result};
use crate::types::StoredTaskSteps;

pub(super) fn parse_task_steps_json(
    steps_json: Option<&str>,
) -> Result<Vec<aster_forge_tasks::TaskStepInfo>> {
    match steps_json {
        Some(raw) if !raw.trim().is_empty() => serde_json::from_str(raw)
            .map_aster_err_ctx("parse task steps json", AsterError::internal_error),
        _ => Ok(Vec::new()),
    }
}

pub(super) fn serialize_task_steps(
    steps: &[aster_forge_tasks::TaskStepInfo],
) -> Result<StoredTaskSteps> {
    serde_json::to_string(steps)
        .map(StoredTaskSteps)
        .map_aster_err_ctx("serialize task steps", AsterError::internal_error)
}

#[cfg(test)]
mod tests {
    use super::{parse_task_steps_json, serialize_task_steps};
    use aster_forge_tasks::{TaskStepInfo, TaskStepStatus};

    fn step(key: &str, status: TaskStepStatus) -> TaskStepInfo {
        TaskStepInfo {
            key: key.to_string(),
            title: key.to_string(),
            status,
            progress_current: 0,
            progress_total: 1,
            detail: None,
            started_at: None,
            finished_at: None,
        }
    }

    #[test]
    fn parse_steps_json_accepts_missing_blank_and_valid_json() {
        assert!(parse_task_steps_json(None).unwrap().is_empty());
        assert!(parse_task_steps_json(Some("  ")).unwrap().is_empty());

        let stored = serialize_task_steps(&[step("prepare", TaskStepStatus::Succeeded)]).unwrap();
        let parsed = parse_task_steps_json(Some(stored.as_ref())).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].key, "prepare");
        assert_eq!(parsed[0].status, TaskStepStatus::Succeeded);

        let error = parse_task_steps_json(Some("not json")).unwrap_err();
        assert!(error.message().contains("parse task steps json"));
    }
}
