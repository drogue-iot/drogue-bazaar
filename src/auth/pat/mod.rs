//! Personal access tokens (pat)

use async_trait::async_trait;
use drogue_client::user::v1::authn::{AuthenticationRequest, AuthenticationResponse};
use std::sync::Arc;

use crate::auth::openid::AuthError;
pub use drogue_client::user::v1::authn::AuthenticationRequest as Request;
pub use drogue_client::user::v1::authn::AuthenticationResponse as Response;
pub use drogue_client::user::v1::authn::Outcome;

#[derive(Clone)]
pub struct Authenticator {
    service: Arc<dyn Service>,
}

impl Authenticator {
    pub fn new<S>(service: S) -> Self
    where
        S: Service + 'static,
    {
        Self {
            service: Arc::new(service),
        }
    }
}

impl Authenticator {
    pub async fn authenticate(
        &self,
        request: AuthenticationRequest,
    ) -> Result<AuthenticationResponse, AuthError> {
        self.service.authenticate(request).await
    }
}

/// Personal access token authenticator
#[async_trait]
pub trait Service {
    /// authenticate a personal access token
    async fn authenticate(
        &self,
        request: AuthenticationRequest,
    ) -> Result<AuthenticationResponse, AuthError>;
}

#[async_trait]
impl Service for drogue_client::user::v1::Client {
    async fn authenticate(&self, request: Request) -> Result<AuthenticationResponse, AuthError> {
        Ok(self
            .authenticate_access_token(request)
            .await
            .map_err(|err| AuthError::Internal(err.to_string()))?)
    }
}
