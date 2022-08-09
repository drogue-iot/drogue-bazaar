//! Authorization tooling.

use async_trait::async_trait;

use crate::auth::openid::AuthError;
pub use drogue_client::user::v1::authz::Outcome;

#[async_trait]
pub trait Authorizer: Clone {
    type Request;

    async fn authorize(&self, request: Self::Request) -> Result<Outcome, AuthError>;
}

#[async_trait]
impl Authorizer for drogue_client::user::v1::Client {
    type Request = drogue_client::user::v1::authz::AuthorizationRequest;

    async fn authorize(&self, request: Self::Request) -> Result<Outcome, AuthError> {
        Ok(self
            .authorize(request)
            .await
            .map_err(|err| AuthError::Internal(err.to_string()))?
            .outcome)
    }
}
