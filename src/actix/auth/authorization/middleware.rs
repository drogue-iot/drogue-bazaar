use super::{AuthZ, Context};
use crate::auth::{UserInformation, ANONYMOUS};
use actix_service::{Service, Transform};
use actix_web::{
    dev::{ServiceRequest, ServiceResponse},
    Error, HttpMessage,
};
use futures_util::future::{ok, LocalBoxFuture, Ready};
use std::rc::Rc;

pub struct AuthMiddleware<S> {
    service: Rc<S>,
    authorizer: AuthZ,
}

// 1. Middleware initialization
// Middleware factory is `Transform` trait from actix-service crate
// `S` - type of the next service
// `B` - type of response's body
impl<S, B> Transform<S, ServiceRequest> for AuthZ
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = S::Error;
    type Transform = AuthMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthMiddleware {
            service: Rc::new(service),
            authorizer: self.clone(),
        })
    }
}

// 2. Middleware's call method gets called with normal request.
impl<S, B> Service<ServiceRequest> for AuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    actix_service::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let srv = Rc::clone(&self.service);
        let auth = self.authorizer.clone();

        Box::pin(async move {
            let result = {
                // extract user information and application from the request
                let ext = req.extensions();
                let identity = ext.get::<UserInformation>().unwrap_or(&ANONYMOUS);

                let context = Context {
                    identity: &identity,
                    request: &req,
                };

                auth.authorize(context).await
            };

            match result {
                Ok(()) => {
                    // forward request to the next service
                    srv.call(req).await
                }
                Err(e) => Err(e.into()),
            }
        })
    }
}
