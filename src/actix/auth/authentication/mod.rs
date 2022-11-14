mod middleware;

use crate::auth::{openid, pat, AuthError, UserInformation};
use ::openid::{Claims, CustomClaims};
use chrono::{DateTime, LocalResult, TimeZone, Utc};
pub use middleware::AuthenticatedUntil;
use tracing::instrument;

/// Credentials
pub enum Credentials {
    /// openID token
    OpenIDToken(String),
    /// username + Personal Access Token
    AccessToken(UsernameAndToken),
    /// Anonymous
    Anonymous,
}

pub struct UsernameAndToken {
    pub username: String,
    pub access_token: Option<String>,
}

/// An Authentication middleware for actix-web relying on drogue-cloud user-auth-service and an openID service
///
/// This middleware will act on each request and try to authenticate the request with :
/// - The `Authorisation: Bearer` header, which should contain an openID token.
/// - The `Authorisation: Basic` header, which should contain a username and an access token issued by the drogue-cloud API.
/// - The `token` query parameter, which should contain am openID token.
///
/// If more than one of the above is provided, the request will be responded with `400: Bad request.`
///
/// After the authentication is successful, this middleware will inject the `UserInformation` in the request object and forward it.
///
/// # Fields
///
/// * `open_id` - An instance of [`openid::Authenticator`] It's an openID client. It is used to verify OpenID tokens.
/// * `token` - An instance of [`pat::Authenticator`]. It's a client for drogue-cloud-user-auth-service. It is used to verify API keys.
/// * `enable_access_token` - Whether to allow access tokens for authentication.
///
#[derive(Clone, Debug)]
pub enum AuthN {
    /// Authentication is disabled, all requests will be using [`UserInformation::Anonymous`].
    Disabled,
    /// Authentication is enabled, using openid or API tokens.
    ///
    /// **NOTE:** If neither is provided, all requests will fail.
    Enabled {
        openid: Option<openid::Authenticator>,
        token: Option<pat::Authenticator>,
    },
}

/// Map a combination of openid and PAT authenticator
impl From<(Option<openid::Authenticator>, Option<pat::Authenticator>)> for AuthN {
    fn from(auth: (Option<openid::Authenticator>, Option<pat::Authenticator>)) -> Self {
        let (openid, token) = auth;
        if openid.is_none() {
            AuthN::Disabled
        } else {
            AuthN::Enabled { openid, token }
        }
    }
}

impl AuthN {
    #[instrument(skip_all, err)]
    async fn authenticate(
        &self,
        credentials: Credentials,
    ) -> Result<(UserInformation, Option<DateTime<Utc>>), AuthError> {
        match self {
            Self::Disabled => {
                // authentication disabled
                Ok((UserInformation::Anonymous, None))
            }
            Self::Enabled { openid, token } => match credentials {
                Credentials::AccessToken(creds) => {
                    if let Some(token) = token {
                        if creds.access_token.is_none() {
                            log::debug!("Cannot authenticate : empty access token.");
                            return Err(AuthError::InvalidRequest(String::from(
                                "No access token provided.",
                            )));
                        }
                        let auth_response = token
                            .authenticate(pat::Request {
                                user_id: creds.username.clone(),
                                access_token: creds.access_token.clone().unwrap_or_default(),
                            })
                            .await
                            .map_err(|e| AuthError::Internal(e.to_string()))?;
                        match auth_response.outcome {
                            pat::Outcome::Known(details) => {
                                Ok((UserInformation::Authenticated(details), None))
                            }
                            pat::Outcome::Unknown => {
                                log::debug!("Unknown access token");
                                Err(AuthError::Forbidden)
                            }
                        }
                    } else {
                        log::debug!("Access token authentication disabled");
                        Err(AuthError::InvalidRequest(
                            "Access token authentication disabled".to_string(),
                        ))
                    }
                }
                Credentials::OpenIDToken(token) => {
                    if let Some(openid) = openid {
                        match openid.validate_token(&token).await {
                            Ok(token) => Ok((
                                UserInformation::Authenticated(token.clone().into()),
                                Some(to_expiration(token.standard_claims().exp())?),
                            )),
                            Err(err) => {
                                log::debug!("Authentication error: {err}");
                                Err(AuthError::Forbidden)
                            }
                        }
                    } else {
                        log::debug!("Open ID authentication disabled");
                        Err(AuthError::InvalidRequest(
                            "Open ID authentication disabled".to_string(),
                        ))
                    }
                }
                Credentials::Anonymous => Ok((UserInformation::Anonymous, None)),
            },
        }
    }
}

/// Convert "exp" timestamp to `DateTime`.
fn to_expiration(exp: i64) -> Result<DateTime<Utc>, AuthError> {
    match Utc.timestamp_opt(exp, 0) {
        LocalResult::None => Err(AuthError::Internal(
            "Unable to convert timestamp".to_string(),
        )),
        LocalResult::Single(exp) => Ok(exp),
        LocalResult::Ambiguous(min, _) => Ok(min),
    }
}
