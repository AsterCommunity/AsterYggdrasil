//! JWT authentication middleware.

use actix_web::{
    Error, HttpMessage,
    dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
    web,
};
use futures::future::{LocalBoxFuture, Ready, ok};
use std::rc::Rc;

use crate::api::middleware::csrf;
use crate::errors::AsterError;
use crate::runtime::AppState;
use crate::services::auth::local;
use crate::services::auth::request::access_cookie_token;
use aster_forge_middleware::actix::csrf::{RequestSourceMode, is_unsafe_method};

/// Authentication middleware that resolves the JWT access cookie into an
/// authenticated user for the wrapped routes.
///
/// For unsafe methods with an access cookie present it additionally enforces
/// CSRF source and double-submit token checks before resolving the user. The
/// resolved user is stored in the request extensions; routes without a valid
/// session fail with the mapped auth error.
pub struct JwtAuth;

impl<S, B> Transform<S, ServiceRequest> for JwtAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = JwtAuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(JwtAuthMiddleware {
            service: Rc::new(service),
        })
    }
}

/// Service produced by [`JwtAuth`]; performs the per-request authentication.
pub struct JwtAuthMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for JwtAuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let svc = self.service.clone();

        Box::pin(async move {
            let state = req
                .app_data::<web::Data<AppState>>()
                .ok_or_else(|| AsterError::internal_error("AppState not found"))?;
            if access_cookie_token(req.request()).is_some() && is_unsafe_method(req.method()) {
                csrf::ensure_service_request_source_allowed(
                    &req,
                    state.get_ref().runtime_config(),
                    RequestSourceMode::OptionalWhenPresent,
                )?;
                csrf::ensure_service_double_submit_token(&req)?;
            }
            let user = local::current_user(state.get_ref(), req.request()).await?;
            let user = local::auth_request_user_info(state.get_ref(), user).await?;

            tracing::Span::current().record("user_id", user.id);
            req.extensions_mut().insert(user);

            svc.call(req).await
        })
    }
}
