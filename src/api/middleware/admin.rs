//! Admin authorization middleware.

use actix_web::{
    Error, HttpMessage,
    dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
};
use futures::future::{LocalBoxFuture, Ready, ok};
use std::rc::Rc;

use crate::errors::AsterError;
use crate::services::auth_service::AuthUserInfo;
use crate::types::{OperatorScope, UserRole};

pub struct RequireAdmin;

#[derive(Clone, Copy)]
pub struct RequireAdminOrScope {
    scope: OperatorScope,
}

impl RequireAdminOrScope {
    pub const fn new(scope: OperatorScope) -> Self {
        Self { scope }
    }
}

impl<S, B> Transform<S, ServiceRequest> for RequireAdmin
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = RequireAdminMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(RequireAdminMiddleware {
            service: Rc::new(service),
        })
    }
}

impl<S, B> Transform<S, ServiceRequest> for RequireAdminOrScope
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = RequireAdminOrScopeMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(RequireAdminOrScopeMiddleware {
            service: Rc::new(service),
            scope: self.scope,
        })
    }
}

pub struct RequireAdminOrScopeMiddleware<S> {
    service: Rc<S>,
    scope: OperatorScope,
}

pub struct RequireAdminMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for RequireAdminOrScopeMiddleware<S>
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
        let scope = self.scope;

        Box::pin(async move {
            let allowed = {
                let extensions = req.extensions();
                let Some(user) = extensions.get::<AuthUserInfo>() else {
                    return Err(AsterError::internal_error(
                        "missing authenticated user in request context",
                    )
                    .into());
                };
                user.role.is_admin()
                    || (user.role == UserRole::Operator && user.operator_scopes.contains(&scope))
            };

            if !allowed {
                return Err(AsterError::auth_admin_required("admin permission required").into());
            }

            svc.call(req).await
        })
    }
}

impl<S, B> Service<ServiceRequest> for RequireAdminMiddleware<S>
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
            let is_admin = {
                let extensions = req.extensions();
                let Some(user) = extensions.get::<AuthUserInfo>() else {
                    return Err(AsterError::internal_error(
                        "missing authenticated user in request context",
                    )
                    .into());
                };
                user.role.is_admin()
            };

            if !is_admin {
                return Err(AsterError::auth_admin_required("admin role required").into());
            }

            svc.call(req).await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::profile_service::{AvatarInfo, UserProfileInfo};
    use crate::types::{AvatarSource, UserStatus};
    use actix_web::{HttpResponse, dev::fn_service, test};

    fn auth_user(role: UserRole, operator_scopes: Vec<OperatorScope>) -> AuthUserInfo {
        AuthUserInfo {
            id: 42,
            username: "scope-user".to_string(),
            email: "scope-user@example.com".to_string(),
            email_verified: true,
            pending_email: None,
            role,
            operator_scopes,
            status: UserStatus::Active,
            must_change_password: false,
            profile: UserProfileInfo {
                display_name: None,
                avatar: AvatarInfo {
                    source: AvatarSource::None,
                    url_512: None,
                    url_1024: None,
                    version: 0,
                },
            },
        }
    }

    async fn call_scope_gate(user: AuthUserInfo) -> Result<u16, Error> {
        let service = fn_service(|req: ServiceRequest| async move {
            Ok(req.into_response(HttpResponse::Ok().finish()))
        });
        let gate = RequireAdminOrScope::new(OperatorScope::TextureLibrary)
            .new_transform(service)
            .await
            .expect("scope gate should build");
        let req = test::TestRequest::default().to_srv_request();
        req.extensions_mut().insert(user);
        gate.call(req)
            .await
            .map(|response| response.status().as_u16())
    }

    #[actix_web::test]
    async fn admin_or_scope_gate_allows_admin_without_scope_rows() {
        let status = call_scope_gate(auth_user(UserRole::Admin, Vec::new()))
            .await
            .unwrap();
        assert_eq!(status, 200);
    }

    #[actix_web::test]
    async fn admin_or_scope_gate_allows_operator_with_matching_scope() {
        let status = call_scope_gate(auth_user(
            UserRole::Operator,
            vec![OperatorScope::TextureLibrary],
        ))
        .await
        .unwrap();
        assert_eq!(status, 200);
    }

    #[actix_web::test]
    async fn admin_or_scope_gate_rejects_operator_without_matching_scope_and_user() {
        let operator_error =
            call_scope_gate(auth_user(UserRole::Operator, vec![OperatorScope::Users]))
                .await
                .unwrap_err();
        assert!(
            operator_error
                .to_string()
                .contains("admin permission required")
        );

        let user_error = call_scope_gate(auth_user(UserRole::User, Vec::new()))
            .await
            .unwrap_err();
        assert!(user_error.to_string().contains("admin permission required"));
    }
}
