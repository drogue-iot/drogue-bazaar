use super::{AuthN, Credentials, UsernameAndToken};
use crate::auth::AuthError;
use actix_http::body::EitherBody;
use actix_service::{Service, Transform};
use actix_web::{
    dev::{ServiceRequest, ServiceResponse},
    web::Query,
    Error, HttpMessage,
};
use actix_web_httpauth::extractors::{basic::BasicAuth, bearer::BearerAuth};
use chrono::{DateTime, Utc};
use futures_util::future::{ok, LocalBoxFuture, Ready};
use serde::Deserialize;
use std::rc::Rc;

pub struct AuthMiddleware<S> {
    service: Rc<S>,
    authenticator: AuthN,
}

// 1. Middleware initialization
// Middleware factory is `Transform` trait from actix-service crate
// `S` - type of the next service
// `B` - type of response's body
impl<S, B> Transform<S, ServiceRequest> for AuthN
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = S::Error;
    type Transform = AuthMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthMiddleware {
            service: Rc::new(service),
            authenticator: self.clone(),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AuthenticatedUntil(pub DateTime<Utc>);

#[derive(Deserialize, Debug)]
struct Token {
    token: String,
}
#[derive(Deserialize, Debug)]
struct ApiKey {
    username: String,
    api_key: String,
}

// 2. Middleware's call method gets called with normal request.
impl<S, B> Service<ServiceRequest> for AuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    actix_service::forward_ready!(service);

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        let srv = Rc::clone(&self.service);
        let auth = self.authenticator.clone();

        Box::pin(async move {
            let basic_auth = req.extract::<BasicAuth>().await;
            let bearer_auth = req.extract::<BearerAuth>().await;

            //let basic_auth = BasicAuth::from_service_request(&req).await;
            //let bearer_auth = BearerAuth::from_service_request(&req).await;

            // This match a "token" or "api_key" query parameter
            let query_str = req.query_string();
            let token_query_param = Query::<Token>::from_query(query_str);
            let api_key_query_param = Query::<ApiKey>::from_query(query_str);

            log::debug!(
                "Basic: {:?}, Bearer: {:?}, Query.token: {:?} Query.api_key: {:?}",
                basic_auth,
                bearer_auth,
                token_query_param,
                api_key_query_param,
            );

            // now evaluate

            let credentials = match (
                basic_auth,
                bearer_auth,
                token_query_param,
                api_key_query_param,
            ) {
                // basic auth is present
                (Ok(basic), Err(_), Err(_), Err(_)) => {
                    Ok(Credentials::AccessToken(UsernameAndToken {
                        username: basic.user_id().to_string(),
                        access_token: basic.password().map(|k| k.to_string()),
                    }))
                }
                // bearer auth is present
                (Err(_), Ok(bearer), Err(_), Err(_)) => {
                    Ok(Credentials::OpenIDToken(bearer.token().to_string()))
                }

                // token query param is present
                (Err(_), Err(_), Ok(query), Err(_)) => Ok(Credentials::OpenIDToken(query.0.token)),
                // api_key query param is present
                (Err(_), Err(_), Err(_), Ok(query)) => {
                    Ok(Credentials::AccessToken(UsernameAndToken {
                        username: query.0.username,
                        access_token: Some(query.0.api_key),
                    }))
                }

                // No headers and no query param (or both headers are invalid, but both invalid should be met with a Bad request anyway)
                (Err(_), Err(_), Err(_query), Err(_api_key)) => Ok(Credentials::Anonymous),
                // More than one way of authentication provided
                // Note on both headers provided and valid -> This never happens, the NGINX load balancer sends back 400 Bad request.
                (_, _, _, _) => Err(AuthError::InvalidRequest(
                    "More than one way of authentication provided".to_string(),
                )),
            };

            // authentication
            let auth_result = match credentials {
                Ok(c) => auth.authenticate(c).await,
                Err(err) => {
                    log::info!("Credentials error: {err}");
                    Err(err)
                }
            };

            match auth_result {
                Ok((user, time)) => {
                    log::debug!("Authenticated: {user:?}");
                    // insert the UserInformation and the expiration time of the token in the request
                    req.extensions_mut().insert(user);
                    if let Some(exp) = time {
                        req.extensions_mut().insert(AuthenticatedUntil(exp));
                    }
                    // then forward it to the next service
                    srv.call(req).await.map(|res| res.map_into_left_body())
                }
                Err(err) => {
                    log::debug!("Authentication error: {err}");
                    Ok(req.error_response(err).map_into_right_body())
                }
            }
        })
    }
}
