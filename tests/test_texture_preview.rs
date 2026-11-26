//! 集成测试：texture preview 端点的后台生成契约（202 / 200 / 404）。
//!
//! 请求路径零渲染：cache miss 入队 `texture_preview_generate` 并回答 202，
//! worker 侧 dispatch 执行后才返回 200 immutable PNG。

#[macro_use]
mod common;

use actix_web::test;
use aster_yggdrasil::entities::background_task;
use aster_yggdrasil::services::texture::{self};
use aster_yggdrasil::types::task::BackgroundTaskStatus;
use sea_orm::EntityTrait;

const STEVE_HASH: &str = "082fdbe1403d09fcf030464bf754439ee79e9287bb15efbe2f54d02f60108133";

fn preview_url(state: &aster_yggdrasil::runtime::AppState, hash: &str) -> String {
    texture::current_texture_preview_url(
        state.runtime_config(),
        hash,
        aster_yggdrasil::types::yggdrasil::MinecraftTextureType::Skin,
        aster_yggdrasil::types::yggdrasil::MinecraftTextureModel::Default,
    )
    .expect("steve hash must map to a preview url")
}

async fn pending_preview_tasks(
    state: &aster_yggdrasil::runtime::AppState,
) -> Vec<background_task::Model> {
    background_task::Entity::find()
        .all(state.writer_db())
        .await
        .expect("background task query should succeed")
}

#[actix_web::test]
async fn preview_cache_miss_enqueues_task_and_answers_202() {
    let (state, _storage_guard) = common::setup_with_isolated_storage().await;
    let app = create_test_app!(state.clone());
    let url = preview_url(&state, STEVE_HASH);

    let req = test::TestRequest::get().uri(&url).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 202);
    assert_eq!(
        resp.headers()
            .get("Retry-After")
            .and_then(|value| value.to_str().ok()),
        Some("2"),
        "202 must carry Retry-After for clients"
    );
    assert_eq!(resp.headers().get("Content-Type"), None);

    let tasks = pending_preview_tasks(&state).await;
    assert_eq!(tasks.len(), 1, "cache miss must enqueue exactly one task");
    assert_eq!(
        tasks[0].kind,
        aster_yggdrasil::types::task::BackgroundTaskKind::TexturePreviewGenerate
    );
    assert_eq!(tasks[0].status, BackgroundTaskStatus::Pending);

    // A second miss dedupes onto the active task instead of enqueueing again.
    let req = test::TestRequest::get().uri(&url).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 202);
    assert_eq!(pending_preview_tasks(&state).await.len(), 1);
}

#[actix_web::test]
async fn worker_dispatch_renders_and_the_next_request_is_200_immutable() {
    let (state, _storage_guard) = common::setup_with_isolated_storage().await;
    let app = create_test_app!(state.clone());
    let url = preview_url(&state, STEVE_HASH);

    let req = test::TestRequest::get().uri(&url).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 202);

    // Worker side: the queue consumer claims and executes the preview task.
    let outcomes = common::run_pending_tasks_once(&state, 8)
        .await
        .expect("dispatch should run");
    assert!(
        !outcomes.is_empty(),
        "the queued preview task must dispatch"
    );
    let tasks = pending_preview_tasks(&state).await;
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].status, BackgroundTaskStatus::Succeeded);
    assert!(
        tasks[0].result_json.is_some(),
        "the render must record its preview file name as the task result"
    );

    let req = test::TestRequest::get().uri(&url).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers()
            .get("Cache-Control")
            .and_then(|value| value.to_str().ok()),
        Some(texture::TEXTURE_PREVIEW_CACHE_CONTROL),
        "cached previews must be immutable"
    );
    let body = test::read_body(resp).await;
    assert!(body.starts_with(b"\x89PNG"), "preview must be a PNG");
}

#[actix_web::test]
async fn unknown_hash_and_stale_file_name_return_404_without_enqueueing() {
    let (state, _storage_guard) = common::setup_with_isolated_storage().await;
    let app = create_test_app!(state.clone());

    let unknown = format!(
        "/api/v1/texture-previews/{}/stale.png",
        "1111111111111111111111111111111111111111111111111111111111111111"
    );
    let resp = test::call_service(&app, test::TestRequest::get().uri(&unknown).to_request()).await;
    assert_eq!(resp.status(), 404);

    let valid_url = preview_url(&state, STEVE_HASH);
    let stale_url = valid_url.replace(".png", "-stale.png");
    let resp =
        test::call_service(&app, test::TestRequest::get().uri(&stale_url).to_request()).await;
    assert_eq!(resp.status(), 404, "spec-mismatched file names must 404");

    assert!(
        pending_preview_tasks(&state).await.is_empty(),
        "404 paths must not enqueue generation tasks"
    );
}

#[actix_web::test]
async fn failed_generation_answers_404_instead_of_202() {
    let (state, _storage_guard) = common::setup_with_isolated_storage().await;
    let app = create_test_app!(state.clone());
    let url = preview_url(&state, STEVE_HASH);

    let resp = test::call_service(&app, test::TestRequest::get().uri(&url).to_request()).await;
    assert_eq!(resp.status(), 202);

    let tasks = pending_preview_tasks(&state).await;
    assert_eq!(tasks.len(), 1);
    use sea_orm::{ActiveModelTrait, Set, Unchanged};
    let mut failed = background_task::ActiveModel {
        id: Unchanged(tasks[0].id),
        ..Default::default()
    };
    failed.status = Set(BackgroundTaskStatus::Failed);
    failed.last_error = Set(Some("source png is corrupt".to_string()));
    failed
        .update(state.writer_db())
        .await
        .expect("failure update should succeed");

    let resp = test::call_service(&app, test::TestRequest::get().uri(&url).to_request()).await;
    assert_eq!(
        resp.status(),
        404,
        "failed generation must stop the retry loop"
    );
    assert_eq!(
        pending_preview_tasks(&state).await.len(),
        1,
        "a failed task must not be re-enqueued by request traffic"
    );
}

#[actix_web::test]
async fn spawned_queue_consumer_executes_enqueued_preview_and_reports_ready() {
    let (state, _storage_guard) = common::setup_with_isolated_storage().await;
    let app = create_test_app!(state.clone());
    let url = preview_url(&state, STEVE_HASH);

    let req = test::TestRequest::get().uri(&url).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 202);

    let statuses = aster_yggdrasil::services::ops::task::consumer_statuses();
    assert_eq!(statuses.len(), 1, "one fallback lane, one runner status");
    let stop = tokio_util::sync::CancellationToken::new();
    let mut tasks = aster_forge_tasks::BackgroundTasks::new();
    aster_yggdrasil::services::ops::task::spawn_consumers(
        actix_web::web::Data::new(state.clone()),
        statuses.clone(),
        stop.clone(),
        &mut tasks,
    );

    // The runner's startup claim must pick the enqueued task up without any
    // explicit wakeup and drive it to a successful preview render.
    let rendered = tokio::time::timeout(std::time::Duration::from_secs(15), async {
        loop {
            let tasks = pending_preview_tasks(&state).await;
            if tasks
                .first()
                .is_some_and(|task| task.status == BackgroundTaskStatus::Succeeded)
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    })
    .await;
    assert!(
        rendered.is_ok(),
        "spawned consumer must execute the queued preview task"
    );

    let task = &pending_preview_tasks(&state).await[0];
    assert!(
        task.result_json.is_some(),
        "preview result must record the rendered file name"
    );
    assert!(
        statuses
            .iter()
            .all(aster_forge_tasks::TaskRuntimeStatus::ready),
        "a claiming runner must report ready for health probes"
    );

    stop.cancel();
    tasks.shutdown().await;
    assert!(
        statuses.iter().all(|status| !status.ready()),
        "stopped runners must leave readiness so orchestrators rotate the instance"
    );
}
