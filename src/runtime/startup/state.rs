use std::sync::Arc;

use crate::config::Config;
use crate::errors::Result;
use crate::runtime::AppState;

use super::common::prepare_common_state;

pub struct PreparedRuntimeState {
    pub state: AppState,
}

pub async fn prepare_runtime_state(config: Arc<Config>) -> Result<PreparedRuntimeState> {
    let state = prepare_common_state(config).await?;

    tracing::info!("runtime state startup complete");

    Ok(PreparedRuntimeState { state })
}
