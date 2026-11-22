//! HTTP runtime component construction.
//!
//! This module owns the Actix server assembly for the product API layer. The
//! runtime entrypoint provides already prepared state and runtime handles, while
//! this module turns them into the Forge service component consumed by
//! `AsterRuntime`.

use std::io;

use actix_web::{App, HttpServer, middleware, web};
use aster_forge_runtime::{
    RuntimeComponentKind, RuntimeServiceComponent, TryRuntimeComponentWithShutdown,
};
use tokio_util::sync::CancellationToken;

/// Configuration needed to build the HTTP service component.
pub struct HttpRuntimeConfig<'a> {
    /// Bind host.
    pub host: &'a str,
    /// Bind port.
    pub port: u16,
    /// Actix worker count.
    pub workers: usize,
    /// Drain budget for in-flight requests, in seconds.
    ///
    /// Currently shares `worker.drain_timeout_secs` with the background-task
    /// drain (issue #11). They bound different lifecycles — in-flight HTTP
    /// requests vs. background task drain — so if a deployment ever needs to
    /// tune them independently, split this into its own
    /// `http_shutdown_timeout_secs` field instead of stretching both.
    pub shutdown_timeout_secs: u64,
}

/// Builds the HTTP runtime component used by the product entrypoint.
pub fn http_component(
    config: HttpRuntimeConfig<'_>,
    state: web::Data<crate::runtime::AppState>,
    metrics_data: web::Data<dyn aster_forge_metrics::MetricsRecorder>,
) -> TryRuntimeComponentWithShutdown<
    RuntimeServiceComponent<actix_web::dev::Server>,
    impl FnOnce(CancellationToken) -> io::Result<RuntimeServiceComponent<actix_web::dev::Server>>,
    io::Error,
> {
    aster_forge_runtime::try_runtime_component_with_shutdown(move |shutdown_token| {
        build_http_service_component(config, state, shutdown_token, metrics_data)
    })
}

fn build_http_service_component(
    config: HttpRuntimeConfig<'_>,
    state: web::Data<crate::runtime::AppState>,
    shutdown_token: CancellationToken,
    metrics_data: web::Data<dyn aster_forge_metrics::MetricsRecorder>,
) -> io::Result<RuntimeServiceComponent<actix_web::dev::Server>> {
    let shutdown_data = web::Data::new(shutdown_token.clone());
    let prometheus_enabled = state.config().observability.prometheus_enabled();
    let server = HttpServer::new(move || {
        // The Prometheus export resource reads the instance-owned Forge provider
        // from app data. Only inject it when the pull exporter is configured.
        #[cfg(feature = "metrics")]
        let app = match prometheus_enabled
            .then(|| metrics_data.provider())
            .flatten()
        {
            Some(provider) => App::new().app_data(web::Data::new(provider)),
            None => App::new(),
        };
        #[cfg(not(feature = "metrics"))]
        let app = App::new();
        app.wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .wrap(aster_forge_middleware::actix::request_id::RequestIdMiddleware)
            .wrap(crate::api::middleware::cors::RuntimeCors)
            .wrap(aster_forge_middleware::actix::security_headers::default_headers())
            .wrap(aster_forge_middleware::actix::metrics::MetricsMiddleware)
            .app_data(state.clone())
            .app_data(shutdown_data.clone())
            .app_data(metrics_data.clone())
            .configure(|cfg| crate::api::configure(cfg, prometheus_enabled))
    })
    .workers(config.workers)
    .bind((config.host, config.port))?
    .shutdown_timeout(config.shutdown_timeout_secs)
    .disable_signals()
    .run();
    let handle = server.handle();
    let shutdown_timeout_secs = config.shutdown_timeout_secs;
    Ok(RuntimeServiceComponent::new(
        "http",
        RuntimeComponentKind::Core,
        server,
        shutdown_token,
        move || async move {
            tracing::info!(
                shutdown_timeout_secs,
                "stopping http server and draining in-flight requests"
            );
            handle.stop(true).await;
        },
    ))
}
