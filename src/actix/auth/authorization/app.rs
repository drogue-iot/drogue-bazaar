use super::{AuthZ, Authorizer, Context};
use crate::actix::auth::authorization::AuthorizerExt;
use crate::auth::AuthError;
use async_trait::async_trait;
use drogue_client::user::v1::authz::AuthorizationRequest;
use drogue_client::user::{self, v1::authz::Outcome};

/// This authorizer will act on each request and makes sure the user have the corrects rights
/// to act on the application.
/// This middleware relies on extracting the user information from the request, so it should be ran
/// after the authentication middleware, see [AuthN](crate::actix_auth::keycloak:authentication::AuthN).
///
/// # Fields
///
/// * `client` - An instance of [`user::v1::Client`] it's a client for drogue-cloud-user-auth-service.
/// * `permission` - The Permission to check. See [Permission](drogue_cloud_service_api::auth::user::authz::Permission) enum.
/// * `app_param` - The name of the application param to extract the value from the request.
///
#[derive(Clone, Debug)]
pub struct ApplicationAuthorizer {
    pub client: user::v1::Client,
    pub permission: user::v1::authz::Permission,
    pub app_param: String,
}

impl ApplicationAuthorizer {
    /// Create a ready-to-use [`AuthZ`] instance for wrapping endpoints. Expects the endpoint to
    /// have an `application` parameter, otherwise the request will be rejected.
    ///
    /// If no "user auth" client is provided, then all requests will pass.
    pub fn wrapping(
        user_auth: Option<user::v1::Client>,
        permission: user::v1::authz::Permission,
    ) -> AuthZ {
        AuthZ::new(
            user_auth
                .map(|client| ApplicationAuthorizer {
                    client,
                    permission,
                    app_param: "application".to_string(),
                })
                // if we don't have a user_auth, we allow everything
                .or_else_allow(),
        )
    }
}

#[async_trait(?Send)]
impl Authorizer for ApplicationAuthorizer {
    async fn authorize(&self, context: &Context<'_>) -> Result<Option<Outcome>, AuthError> {
        let user = &context.identity;

        let application = match context.request.match_info().get(&self.app_param) {
            Some(application) => application,
            None => {
                // we are missing information, we won't process
                return Err(AuthError::InvalidRequest(
                    "Missing 'application' information".to_string(),
                ));
            }
        };

        log::debug!(
            "Authorizing - user: {:?}, app: {}, permission: {:?}",
            user,
            application,
            &self.permission
        );

        let response = self
            .client
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
            Outcome::Allow => Ok(Some(Outcome::Allow)),
            Outcome::Deny => Err(AuthError::NotFound(
                String::from("Application"),
                application.to_string(),
            )),
        }
    }
}
