//! Generic administrator API route registration.

use crate::api::middleware::{admin::RequireAdminOrScope, auth::JwtAuth, rate_limit};
use crate::config::{NetworkTrustConfig, RateLimitConfig};
use crate::types::OperatorScope;
use actix_governor::Governor;
use actix_web::middleware::Condition;
use actix_web::web;

pub mod audit_logs;
pub mod avatars;
pub mod config;
pub mod external_auth;
pub mod overview;
pub mod profiles;
pub mod system_info;
pub mod tasks;
pub mod texture_library;
pub mod users;

pub use audit_logs::list_audit_logs;
pub use avatars::get_user_avatar;
pub use config::{
    config_schema, config_template_variables, delete_config, execute_config_action, get_config,
    list_config, set_config,
};
pub use external_auth::{
    create_external_auth_provider, delete_external_auth_provider, get_external_auth_provider,
    list_external_auth_provider_kinds, list_external_auth_providers, test_external_auth_provider,
    test_external_auth_provider_params, update_external_auth_provider,
};
pub use overview::get_overview;
pub use profiles::{
    delete_minecraft_profile, delete_minecraft_profile_texture, delete_minecraft_textures_by_hash,
    get_minecraft_profile, list_minecraft_profile_textures, list_minecraft_profiles,
    list_user_minecraft_profiles, rename_minecraft_profile,
};
pub use system_info::get_system_info;
pub use tasks::{cleanup_tasks, list_tasks, retry_task};
pub use texture_library::{
    create_texture_library_tag, delete_texture_library_tag, list_texture_library_tags,
    update_texture_library_tag,
};
pub use users::{
    create_user, create_user_invitation, delete_user, get_user, list_user_invitations, list_users,
    revoke_user_invitation, revoke_user_sessions, update_user,
};

pub fn routes(
    rl: &RateLimitConfig,
    network_trust: &NetworkTrustConfig,
) -> impl actix_web::dev::HttpServiceFactory + use<> {
    let limiter = rate_limit::build_governor(&rl.write, &network_trust.trusted_proxies);

    web::scope("/admin")
        .wrap(Condition::new(rl.enabled, Governor::new(&limiter)))
        .service(
            web::scope("")
                .wrap(JwtAuth)
                .service(
                    web::scope("/overview")
                        .wrap(RequireAdminOrScope::new(OperatorScope::Overview))
                        .route("", web::get().to(get_overview)),
                )
                .service(
                    web::scope("/system-info")
                        .wrap(RequireAdminOrScope::new(OperatorScope::Overview))
                        .route("", web::get().to(get_system_info)),
                )
                .service(
                    web::scope("/config")
                        .wrap(RequireAdminOrScope::new(OperatorScope::Settings))
                        .route("", web::get().to(list_config))
                        .route("/schema", web::get().to(config_schema))
                        .route(
                            "/template-variables",
                            web::get().to(config_template_variables),
                        )
                        .route("/{key}", web::get().to(get_config))
                        .route("/{key}", web::put().to(set_config))
                        .route("/{key}", web::delete().to(delete_config))
                        .route("/{key}/action", web::post().to(execute_config_action)),
                )
                .service(
                    web::scope("/audit-logs")
                        .wrap(RequireAdminOrScope::new(OperatorScope::Audit))
                        .route("", web::get().to(list_audit_logs)),
                )
                .service(
                    web::scope("/tasks")
                        .wrap(RequireAdminOrScope::new(OperatorScope::Tasks))
                        .route("", web::get().to(list_tasks))
                        .route("/cleanup", web::post().to(cleanup_tasks))
                        .route("/{id}/retry", web::post().to(retry_task)),
                )
                .service(
                    web::scope("/texture-library")
                        .wrap(RequireAdminOrScope::new(OperatorScope::TextureLibrary))
                        .route("/tags", web::get().to(list_texture_library_tags))
                        .route("/tags", web::post().to(create_texture_library_tag))
                        .route(
                            "/tags/{tag_id}",
                            web::patch().to(update_texture_library_tag),
                        )
                        .route(
                            "/tags/{tag_id}",
                            web::delete().to(delete_texture_library_tag),
                        ),
                )
                .service(
                    web::scope("/users")
                        .wrap(RequireAdminOrScope::new(OperatorScope::Users))
                        .route("", web::get().to(list_users))
                        .route("", web::post().to(create_user))
                        .route("/invitations", web::get().to(list_user_invitations))
                        .route("/invitations", web::post().to(create_user_invitation))
                        .route(
                            "/invitations/{id}/revoke",
                            web::post().to(revoke_user_invitation),
                        )
                        .route("/{id}", web::get().to(get_user))
                        .route("/{id}", web::patch().to(update_user))
                        .route("/{id}", web::delete().to(delete_user))
                        .route(
                            "/{id}/sessions/revoke",
                            web::post().to(revoke_user_sessions),
                        )
                        .route(
                            "/{user_id}/minecraft-profiles",
                            web::get().to(list_user_minecraft_profiles),
                        ),
                )
                .service(
                    web::scope("/avatars")
                        .wrap(RequireAdminOrScope::new(OperatorScope::Users))
                        .route("/users/{id}/{size}", web::get().to(get_user_avatar)),
                )
                .service(
                    web::scope("/minecraft-profiles")
                        .wrap(RequireAdminOrScope::new(OperatorScope::Profiles))
                        .route("", web::get().to(list_minecraft_profiles))
                        .route("/{uuid}", web::get().to(get_minecraft_profile))
                        .route("/{uuid}/name", web::put().to(rename_minecraft_profile))
                        .route("/{uuid}", web::delete().to(delete_minecraft_profile))
                        .route(
                            "/{uuid}/textures",
                            web::get().to(list_minecraft_profile_textures),
                        )
                        .route(
                            "/{uuid}/textures/{texture_type}",
                            web::delete().to(delete_minecraft_profile_texture),
                        ),
                )
                .service(
                    web::scope("/minecraft-textures")
                        .wrap(RequireAdminOrScope::new(OperatorScope::Profiles))
                        .route(
                            "/{hash}",
                            web::delete().to(delete_minecraft_textures_by_hash),
                        ),
                )
                .service(
                    web::scope("/external-auth")
                        .wrap(RequireAdminOrScope::new(OperatorScope::ExternalAuth))
                        .route(
                            "/provider-kinds",
                            web::get().to(list_external_auth_provider_kinds),
                        )
                        .route("/providers", web::get().to(list_external_auth_providers))
                        .route("/providers", web::post().to(create_external_auth_provider))
                        .route(
                            "/providers/test",
                            web::post().to(test_external_auth_provider_params),
                        )
                        .route("/providers/{id}", web::get().to(get_external_auth_provider))
                        .route(
                            "/providers/{id}",
                            web::patch().to(update_external_auth_provider),
                        )
                        .route(
                            "/providers/{id}",
                            web::delete().to(delete_external_auth_provider),
                        )
                        .route(
                            "/providers/{id}/test",
                            web::post().to(test_external_auth_provider),
                        ),
                ),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, http::StatusCode, test};

    async fn route_is_registered(path: &str) -> bool {
        let rate_limit = RateLimitConfig {
            enabled: false,
            ..Default::default()
        };
        let network_trust = NetworkTrustConfig::default();
        let app = test::init_service(App::new().service(routes(&rate_limit, &network_trust))).await;
        let req = test::TestRequest::get().uri(path).to_request();
        match test::try_call_service(&app, req).await {
            Ok(response) => response.status() != StatusCode::NOT_FOUND,
            Err(_) => true,
        }
    }

    #[actix_web::test]
    async fn admin_prefixed_scopes_do_not_shadow_each_other() {
        for path in [
            "/admin/overview",
            "/admin/config",
            "/admin/tasks",
            "/admin/texture-library/tags",
            "/admin/users",
            "/admin/avatars/users/1/512",
            "/admin/minecraft-profiles",
            "/admin/external-auth/providers",
        ] {
            assert!(route_is_registered(path).await, "{path}");
        }
    }
}
