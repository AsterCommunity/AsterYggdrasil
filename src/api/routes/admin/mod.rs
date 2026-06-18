//! Generic administrator API route registration.

use crate::api::middleware::{admin::RequireAdmin, auth::JwtAuth, rate_limit};
use crate::config::{NetworkTrustConfig, RateLimitConfig};
use actix_governor::Governor;
use actix_web::middleware::Condition;
use actix_web::web;

pub mod audit_logs;
pub mod config;
pub mod external_auth;
pub mod profiles;
pub mod system_info;
pub mod tasks;
pub mod users;

pub use audit_logs::list_audit_logs;
pub use config::{
    config_schema, config_template_variables, delete_config, execute_config_action, get_config,
    list_config, set_config,
};
pub use external_auth::{
    create_external_auth_provider, delete_external_auth_provider, get_external_auth_provider,
    list_external_auth_provider_kinds, list_external_auth_providers, test_external_auth_provider,
    test_external_auth_provider_params, update_external_auth_provider,
};
pub use profiles::{
    delete_minecraft_profile, delete_minecraft_profile_texture, delete_minecraft_textures_by_hash,
    get_minecraft_profile, list_minecraft_profile_textures, list_minecraft_profiles,
    list_user_minecraft_profiles, rename_minecraft_profile,
};
pub use system_info::get_system_info;
pub use tasks::{cleanup_tasks, list_tasks, retry_task};
pub use users::{
    create_user, create_user_invitation, get_user, get_user_avatar, list_user_invitations,
    list_users, revoke_user_invitation, revoke_user_sessions, update_user,
};

pub fn routes(
    rl: &RateLimitConfig,
    network_trust: &NetworkTrustConfig,
) -> impl actix_web::dev::HttpServiceFactory + use<> {
    let limiter = rate_limit::build_governor(&rl.write, &network_trust.trusted_proxies);

    web::scope("/admin")
        .wrap(Condition::new(rl.enabled, Governor::new(&limiter)))
        .service(
            web::scope("").wrap(JwtAuth).service(
                web::scope("")
                    .wrap(RequireAdmin)
                    .route("/audit-logs", web::get().to(list_audit_logs))
                    .route("/system-info", web::get().to(get_system_info))
                    .route("/config", web::get().to(list_config))
                    .route("/config/schema", web::get().to(config_schema))
                    .route(
                        "/config/template-variables",
                        web::get().to(config_template_variables),
                    )
                    .route("/config/{key}", web::get().to(get_config))
                    .route("/config/{key}", web::put().to(set_config))
                    .route("/config/{key}", web::delete().to(delete_config))
                    .route(
                        "/config/{key}/action",
                        web::post().to(execute_config_action),
                    )
                    .route("/tasks", web::get().to(list_tasks))
                    .route("/tasks/cleanup", web::post().to(cleanup_tasks))
                    .route("/tasks/{id}/retry", web::post().to(retry_task))
                    .route("/users", web::get().to(list_users))
                    .route("/users", web::post().to(create_user))
                    .route("/users/invitations", web::get().to(list_user_invitations))
                    .route("/users/invitations", web::post().to(create_user_invitation))
                    .route(
                        "/users/invitations/{id}/revoke",
                        web::post().to(revoke_user_invitation),
                    )
                    .route("/users/{id}", web::get().to(get_user))
                    .route("/users/{id}", web::patch().to(update_user))
                    .route("/users/{id}/avatar/{size}", web::get().to(get_user_avatar))
                    .route(
                        "/users/{id}/sessions/revoke",
                        web::post().to(revoke_user_sessions),
                    )
                    .route(
                        "/users/{user_id}/minecraft-profiles",
                        web::get().to(list_user_minecraft_profiles),
                    )
                    .route(
                        "/minecraft-profiles",
                        web::get().to(list_minecraft_profiles),
                    )
                    .route(
                        "/minecraft-profiles/{uuid}",
                        web::get().to(get_minecraft_profile),
                    )
                    .route(
                        "/minecraft-profiles/{uuid}/name",
                        web::put().to(rename_minecraft_profile),
                    )
                    .route(
                        "/minecraft-profiles/{uuid}",
                        web::delete().to(delete_minecraft_profile),
                    )
                    .route(
                        "/minecraft-profiles/{uuid}/textures",
                        web::get().to(list_minecraft_profile_textures),
                    )
                    .route(
                        "/minecraft-profiles/{uuid}/textures/{texture_type}",
                        web::delete().to(delete_minecraft_profile_texture),
                    )
                    .route(
                        "/minecraft-textures/{hash}",
                        web::delete().to(delete_minecraft_textures_by_hash),
                    )
                    .route(
                        "/external-auth/provider-kinds",
                        web::get().to(list_external_auth_provider_kinds),
                    )
                    .route(
                        "/external-auth/providers",
                        web::get().to(list_external_auth_providers),
                    )
                    .route(
                        "/external-auth/providers",
                        web::post().to(create_external_auth_provider),
                    )
                    .route(
                        "/external-auth/providers/test",
                        web::post().to(test_external_auth_provider_params),
                    )
                    .route(
                        "/external-auth/providers/{id}",
                        web::get().to(get_external_auth_provider),
                    )
                    .route(
                        "/external-auth/providers/{id}",
                        web::patch().to(update_external_auth_provider),
                    )
                    .route(
                        "/external-auth/providers/{id}",
                        web::delete().to(delete_external_auth_provider),
                    )
                    .route(
                        "/external-auth/providers/{id}/test",
                        web::post().to(test_external_auth_provider),
                    ),
            ),
        )
}
