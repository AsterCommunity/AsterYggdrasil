//! API 路由：`frontend`。

use crate::api::cache::conditional_bytes_response;
use crate::config::{branding, yggdrasil::DEFAULT_YGGDRASIL_API_ROOT_ALI};
use crate::runtime::AppState;
use actix_web::{HttpRequest, HttpResponse, web};
use rust_embed::Embed;
use std::path::PathBuf;

#[derive(Embed)]
#[folder = "frontend-panel/dist/"]
struct FrontendAssets;

/// 运行时可覆盖的前端目录
const CUSTOM_FRONTEND_DIR: &str = "./frontend-override";
const FILE_NOT_FOUND_MESSAGE: &str = "File not found";
const INDEX_CACHE_CONTROL: &str = "no-cache";
const IMMUTABLE_ASSET_CACHE_CONTROL: &str = "public, max-age=31536000, immutable";
const STATIC_ASSET_CACHE_CONTROL: &str = "public, max-age=86400";
const PWA_CACHE_CONTROL: &str = "no-cache";

pub const FRONTEND_CSP_HEADER: &str = concat!(
    "default-src 'self'; ",
    "base-uri 'self'; ",
    "object-src 'none'; ",
    "frame-ancestors 'self'; ",
    "script-src 'self' 'unsafe-inline'; ",
    "style-src 'self' 'unsafe-inline'; ",
    "img-src 'self' data: blob: http: https:; ",
    "font-src 'self' data:; ",
    // presigned upload / download 可能直接命中外部对象存储或 remote follower，
    // 这里必须允许浏览器向任意 http(s) 终点发起 XHR/fetch/WebSocket 连接。
    "connect-src 'self' http: https: ws: wss:; ",
    "media-src 'self' blob:; ",
    "worker-src 'self' blob:; ",
    "frame-src 'self' http: https:; ",
    "manifest-src 'self'"
);

pub const FRONTEND_CSP_META: &str = concat!(
    "default-src 'self'; ",
    "base-uri 'self'; ",
    "object-src 'none'; ",
    "script-src 'self' 'unsafe-inline'; ",
    "style-src 'self' 'unsafe-inline'; ",
    "img-src 'self' data: blob: http: https:; ",
    "font-src 'self' data:; ",
    // meta CSP 不能承载 frame-ancestors；该约束仍由响应头版 CSP 生效。
    "connect-src 'self' http: https: ws: wss:; ",
    "media-src 'self' blob:; ",
    "worker-src 'self' blob:; ",
    "frame-src 'self' http: https:; ",
    "manifest-src 'self'"
);

pub struct FrontendService;

impl FrontendService {
    /// 优先从自定义目录加载，fallback 到嵌入资源
    async fn load_file(file_path: &str) -> Option<Vec<u8>> {
        if file_path.contains("..") {
            return None;
        }

        let custom_path = PathBuf::from(CUSTOM_FRONTEND_DIR).join(file_path);
        if let Ok(data) = tokio::fs::read(&custom_path).await {
            tracing::trace!("serving from custom dir: {file_path}");
            return Some(data);
        }

        FrontendAssets::get(file_path).map(|c| c.data.into_owned())
    }

    /// 服务 index.html，替换配置占位符
    async fn serve_index(state: &AppState) -> HttpResponse {
        let html = match Self::load_file("index.html").await {
            Some(data) => String::from_utf8_lossy(&data).into_owned(),
            None => include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/frontend-panel/dist/index.html"
            ))
            .to_string(),
        };

        let processed = html
            .replace("%ASTERYGGDRASIL_VERSION%", env!("CARGO_PKG_VERSION"))
            .replace(
                "%ASTERYGGDRASIL_TITLE%",
                &escape_html(branding::title_or_default(state.runtime_config())),
            )
            .replace(
                "%ASTERYGGDRASIL_DESCRIPTION%",
                &escape_html(branding::description_or_default(state.runtime_config())),
            )
            .replace(
                "%ASTERYGGDRASIL_FAVICON_URL%",
                &escape_html(branding::favicon_url_or_default(state.runtime_config())),
            )
            .replace(
                "%ASTERYGGDRASIL_WORDMARK_DARK_URL%",
                &escape_html(branding::wordmark_dark_url_or_default(
                    state.runtime_config(),
                )),
            )
            .replace(
                "%ASTERYGGDRASIL_WORDMARK_LIGHT_URL%",
                &escape_html(branding::wordmark_light_url_or_default(
                    state.runtime_config(),
                )),
            )
            .replace("%ASTERYGGDRASIL_CSP%", &escape_html(FRONTEND_CSP_META));

        HttpResponse::Ok()
            .insert_header(("Content-Security-Policy", FRONTEND_CSP_HEADER))
            .insert_header((
                "X-Authlib-Injector-API-Location",
                DEFAULT_YGGDRASIL_API_ROOT_ALI,
            ))
            .insert_header(("Cache-Control", INDEX_CACHE_CONTROL))
            .content_type("text/html; charset=utf-8")
            .body(processed)
    }

    pub async fn handle_index(state: web::Data<AppState>, _req: HttpRequest) -> HttpResponse {
        Self::serve_index(state.get_ref()).await
    }

    pub async fn handle_assets(req: HttpRequest) -> HttpResponse {
        let path = req.match_info().query("path");
        let asset_path = format!("assets/{path}");
        let content_type = Self::get_content_type(path);

        match Self::load_file(&asset_path).await {
            Some(data) => {
                conditional_bytes_response(&req, data, content_type, IMMUTABLE_ASSET_CACHE_CONTROL)
            }
            None => HttpResponse::NotFound().body(FILE_NOT_FOUND_MESSAGE),
        }
    }

    pub async fn handle_static(req: HttpRequest) -> HttpResponse {
        let path = req.match_info().query("path");
        let asset_path = format!("static/{path}");
        let content_type = Self::get_content_type(path);

        match Self::load_file(&asset_path).await {
            Some(data) => {
                conditional_bytes_response(&req, data, content_type, STATIC_ASSET_CACHE_CONTROL)
            }
            None => HttpResponse::NotFound().body(FILE_NOT_FOUND_MESSAGE),
        }
    }

    pub async fn handle_favicon(req: HttpRequest) -> HttpResponse {
        match Self::load_file("favicon.svg").await {
            Some(data) => {
                conditional_bytes_response(&req, data, "image/svg+xml", STATIC_ASSET_CACHE_CONTROL)
            }
            None => HttpResponse::Ok()
                .insert_header(("Cache-Control", STATIC_ASSET_CACHE_CONTROL))
                .content_type("image/svg+xml")
                .body(Vec::new()),
        }
    }

    pub async fn handle_spa_fallback(
        state: web::Data<AppState>,
        _req: HttpRequest,
    ) -> HttpResponse {
        Self::serve_index(state.get_ref()).await
    }

    pub async fn handle_pwa_file(req: HttpRequest) -> HttpResponse {
        let filename = req.uri().path().trim_start_matches('/');
        let content_type = Self::get_content_type(filename);
        match Self::load_file(filename).await {
            Some(data) => conditional_bytes_response(&req, data, content_type, PWA_CACHE_CONTROL),
            None => HttpResponse::NotFound().body(FILE_NOT_FOUND_MESSAGE),
        }
    }

    fn get_content_type(path: &str) -> &'static str {
        match path.rsplit('.').next() {
            Some("css") => "text/css",
            Some("js" | "mjs") => "application/javascript",
            Some("json") => "application/json",
            Some("webmanifest") => "application/manifest+json",
            Some("png") => "image/png",
            Some("webp") => "image/webp",
            Some("jpg" | "jpeg") => "image/jpeg",
            Some("gif") => "image/gif",
            Some("svg") => "image/svg+xml",
            Some("ico") => "image/x-icon",
            Some("woff") => "font/woff",
            Some("woff2") => "font/woff2",
            Some("ttf") => "font/ttf",
            _ => "application/octet-stream",
        }
    }
}

fn escape_html(value: impl AsRef<str>) -> String {
    value
        .as_ref()
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// 前端路由，挂在 `/` 下，必须最后注册
pub fn routes() -> actix_web::Scope {
    web::scope("")
        .route("/", web::get().to(FrontendService::handle_index))
        .route(
            "/assets/{path:.*}",
            web::get().to(FrontendService::handle_assets),
        )
        .route(
            "/static/{path:.*}",
            web::get().to(FrontendService::handle_static),
        )
        .route(
            "/favicon.svg",
            web::get().to(FrontendService::handle_favicon),
        )
        // PWA 文件（sw.js, workbox-*.js, manifest.webmanifest）
        .route(
            "/registerSW.js",
            web::get().to(FrontendService::handle_pwa_file),
        )
        .route("/sw.js", web::get().to(FrontendService::handle_pwa_file))
        .route(
            "/manifest.webmanifest",
            web::get().to(FrontendService::handle_pwa_file),
        )
        .route(
            "/{filename:workbox-[^/]*}",
            web::get().to(FrontendService::handle_pwa_file),
        )
        // SPA fallback（最后）
        .route(
            "/{path:.*}",
            web::get().to(FrontendService::handle_spa_fallback),
        )
}

#[cfg(test)]
mod tests {
    use super::{FrontendAssets, routes};
    use actix_web::{
        App,
        http::{StatusCode, header},
        test,
    };

    #[actix_web::test]
    async fn asset_requests_do_not_fall_back_to_spa() {
        let app = test::init_service(App::new().service(routes())).await;
        let req = test::TestRequest::get()
            .uri("/assets/__missing_test_asset__.js")
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn hashed_assets_are_served_with_immutable_cache_control() {
        let asset = FrontendAssets::iter()
            .find(|path| path.starts_with("assets/"))
            .expect("frontend dist should include at least one hashed asset");
        let route = asset
            .strip_prefix("assets/")
            .expect("asset path should have assets prefix");
        let app = test::init_service(App::new().service(routes())).await;
        let req = test::TestRequest::get()
            .uri(&format!("/assets/{route}"))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers()
                .get(header::CACHE_CONTROL)
                .and_then(|value| value.to_str().ok()),
            Some("public, max-age=31536000, immutable")
        );
    }

    #[actix_web::test]
    async fn static_assets_are_served_with_short_cache_control() {
        let asset = FrontendAssets::iter()
            .find(|path| path.starts_with("static/"))
            .expect("frontend dist should include at least one static asset");
        let route = asset
            .strip_prefix("static/")
            .expect("asset path should have static prefix");
        let app = test::init_service(App::new().service(routes())).await;
        let req = test::TestRequest::get()
            .uri(&format!("/static/{route}"))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers()
                .get(header::CACHE_CONTROL)
                .and_then(|value| value.to_str().ok()),
            Some("public, max-age=86400")
        );
    }

    #[actix_web::test]
    async fn static_image_requests_support_etag_revalidation() {
        let asset = FrontendAssets::iter()
            .find(|path| path.starts_with("static/") && path.ends_with(".png"))
            .expect("frontend dist should include at least one static image");
        let route = asset
            .strip_prefix("static/")
            .expect("asset path should have static prefix");
        let app = test::init_service(App::new().service(routes())).await;
        let req = test::TestRequest::get()
            .uri(&format!("/static/{route}"))
            .to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::OK);
        let etag = resp
            .headers()
            .get(header::ETAG)
            .and_then(|value| value.to_str().ok())
            .expect("static image response should include etag")
            .to_owned();

        let req = test::TestRequest::get()
            .uri(&format!("/static/{route}"))
            .insert_header((header::IF_NONE_MATCH, etag))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::NOT_MODIFIED);
        assert_eq!(
            resp.headers()
                .get(header::CACHE_CONTROL)
                .and_then(|value| value.to_str().ok()),
            Some("public, max-age=86400")
        );
    }

    #[actix_web::test]
    async fn pwa_files_are_revalidated() {
        let app = test::init_service(App::new().service(routes())).await;
        let req = test::TestRequest::get().uri("/sw.js").to_request();

        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers()
                .get(header::CACHE_CONTROL)
                .and_then(|value| value.to_str().ok()),
            Some("no-cache")
        );
    }

    #[actix_web::test]
    async fn pwa_register_script_does_not_fall_back_to_spa() {
        let app = test::init_service(App::new().service(routes())).await;
        let req = test::TestRequest::get().uri("/registerSW.js").to_request();

        let resp = test::call_service(&app, req).await;
        let status = resp.status();
        let content_type = resp
            .headers()
            .get(actix_web::http::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_owned();

        assert_ne!(status, StatusCode::OK);
        assert!(
            !content_type.starts_with("text/html"),
            "registerSW.js must not be served as SPA HTML"
        );
    }
}
