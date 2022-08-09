use crate::auth::{authz, AuthError, UserInformation};
use drogue_client::user::{self, v1::authz::AuthorizationRequest};

mod middleware;

/// An Authorization middleware for actix-web relying on drogue-cloud user-auth-service.
///
/// This middleware will act on each request and makes sure the user have the corrects rights
/// to act on the application.
/// This middleware relies on extracting the user information from the request, so it should be ran
/// after the authentication middleware, see [AuthN](crate::actix_auth::keycloak:authentication::AuthN).
///
/// # Fields
///
/// * `client` - An instance of `UserAuthClient` it's a client for drogue-cloud-user-auth-service.
/// * `permission` - The Permission to check. See [Permission](drogue_cloud_service_api::auth::user::authz::Permission) enum.
/// * `app_param` - The name of the application param to extract the value from the request.
///
#[derive(Clone)]
pub struct AuthZ {
    pub client: Option<user::v1::Client>,
    pub permission: user::v1::authz::Permission,
    pub app_param: String,
}

impl AuthZ {
    /// Authorise a request
    pub async fn authorize(
        &self,
        application: &str,
        user: UserInformation,
    ) -> Result<(), AuthError> {
        match &self.client {
            Some(client) => {
                log::debug!(
                    "Authorizing - user: {:?}, app: {}, permission: {:?}",
                    user,
                    application,
                    &self.permission
                );

                let response = client
                    .authorize(AuthorizationRequest {
                        application: application.to_string(),
                        permission: self.permission,
                        user_id: user.user_id().map(ToString::to_string),
                        roles: user.roles().clone(),
                    })
                    .await
                    .map_err(|e| AuthError::Internal(e.to_string()))?;

                log::debug!("Outcome: {:?}", response);

                match response.outcome {
                    authz::Outcome::Allow => Ok(()),
                    authz::Outcome::Deny => Err(AuthError::NotFound(
                        String::from("Application"),
                        application.to_string(),
                    )),
                }
            }
            // No auth client
            None => Err(AuthError::Internal(String::from(
                "Missing Authorization client.",
            ))),
        }
    }
}
