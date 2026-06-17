//! API route registration.

pub mod account;
pub mod admin;
pub mod auth;
pub mod auth_external_auth;
pub mod frontend;
pub mod health;
pub mod profiles;
pub mod public;
pub mod wardrobe;
pub mod yggdrasil;

use actix_web::web;

pub fn configure_api(cfg: &mut web::ServiceConfig) {
    cfg.app_data(crate::api::common::project_query_config())
        .configure(auth_external_auth::configure)
        .configure(auth::configure)
        .configure(account::configure)
        .configure(profiles::configure)
        .configure(wardrobe::configure)
        .configure(public::configure)
        .service(admin::routes(
            &crate::config::get_config().rate_limit,
            &crate::config::get_config().network_trust,
        ))
        .default_service(web::to(crate::api::common::api_not_found));
}
