use crate::auth::{AuthError, UserInformation};
use actix_web::dev::ServiceRequest;
use async_trait::async_trait;
use drogue_client::user::v1::authz::Outcome;
use std::sync::Arc;

mod app;
mod middleware;

pub use app::*;

/// An Authorization middleware for actix-web.
///
/// It will ask the authorizer, and if it abstains, rejects the request.

#[derive(Clone)]
pub struct AuthZ {
    pub authorizer: Arc<dyn Authorizer>,
}

impl AuthZ {
    pub fn new<A>(authorizer: A) -> Self
    where
        A: Authorizer + 'static,
    {
        Self {
            authorizer: Arc::new(authorizer),
        }
    }

    /// Authorise a request
    pub async fn authorize(&self, context: Context<'_>) -> Result<(), AuthError> {
        match self.authorizer.authorize(&context).await {
            Ok(Some(Outcome::Allow)) => Ok(()),
            Ok(None) | Ok(Some(Outcome::Deny)) => Err(AuthError::Forbidden),
            Err(err) => Err(err),
        }
    }
}

pub struct Context<'a> {
    pub request: &'a ServiceRequest,
    pub identity: &'a UserInformation,
}

#[async_trait(?Send)]
pub trait Authorizer {
    /// Try to authorize an operation.
    ///
    /// The outcome can be:
    /// * `Ok(None)` -> Abstain: continue evaluating.
    /// * `Ok(Some(outcome))` -> Depending on the outcome, pass or fail right away.
    /// * `Err(err)` -> Some error, abort right away.
    ///
    /// In case all authorizers abstain, the caller has the final call in allowing/rejecting the
    /// request.
    ///
    /// In order to return "not found" instead of "not allowed" it is possible to return
    /// `Err(AuthError::NotFound(..))`.
    async fn authorize(&self, context: &Context<'_>) -> Result<Option<Outcome>, AuthError>;
}

/// Iterate over an array of authorizers and return the first non-abstain result.
#[async_trait(?Send)]
impl Authorizer for Vec<Box<dyn Authorizer>> {
    async fn authorize(&self, context: &Context<'_>) -> Result<Option<Outcome>, AuthError> {
        for a in self {
            match a.authorize(context).await? {
                None => {
                    // keep going
                }
                Some(outcome) => return Ok(Some(outcome)),
            }
        }

        // no one voted
        Ok(None)
    }
}

#[async_trait(?Send)]
impl<A: Authorizer> Authorizer for Option<A> {
    async fn authorize(&self, context: &Context<'_>) -> Result<Option<Outcome>, AuthError> {
        match &self {
            Some(authorizer) => authorizer.authorize(context).await,
            None => Ok(None),
        }
    }
}

/// An authorizer which returns the provided outcome in case the provided authorizer abstained.
pub struct OrElseAuthorizer<A>(A, Outcome)
where
    A: Authorizer;

#[async_trait(?Send)]
impl<A> Authorizer for OrElseAuthorizer<A>
where
    A: Authorizer,
{
    async fn authorize(&self, context: &Context<'_>) -> Result<Option<Outcome>, AuthError> {
        self.0
            .authorize(context)
            .await
            .map(|r| Some(r.unwrap_or_else(|| self.1)))
    }
}

pub struct IntoNotFound<A, F>(A, F)
where
    A: Authorizer,
    F: Fn(&Context) -> (String, String);

#[async_trait(?Send)]
impl<A, F> Authorizer for IntoNotFound<A, F>
where
    A: Authorizer,
    F: Fn(&Context) -> (String, String),
{
    async fn authorize(&self, context: &Context<'_>) -> Result<Option<Outcome>, AuthError> {
        match self.0.authorize(context).await {
            Ok(None) => Ok(None),
            Ok(Some(Outcome::Allow)) => Ok(Some(Outcome::Allow)),
            Ok(Some(Outcome::Deny)) => {
                let (r#type, id) = self.1(context);
                Err(AuthError::NotFound(r#type, id))
            }
            Err(err) => Err(err),
        }
    }
}

pub trait AuthorizerExt: Authorizer + Sized {
    fn or_else(self, outcome: Outcome) -> OrElseAuthorizer<Self> {
        OrElseAuthorizer(self, outcome)
    }

    fn or_else_allow(self) -> OrElseAuthorizer<Self> {
        self.or_else(Outcome::Allow)
    }

    fn or_else_deny(self) -> OrElseAuthorizer<Self> {
        self.or_else(Outcome::Deny)
    }

    fn into_not_found<F>(self, f: F) -> IntoNotFound<Self, F>
    where
        F: Fn(&Context) -> (String, String),
    {
        IntoNotFound(self, f)
    }
}

impl<A> AuthorizerExt for A where A: Authorizer {}

/// An authorizer which rejects [`UserInformation::Anonymous`] identities.
pub struct NotAnonymous;

#[async_trait(?Send)]
impl Authorizer for NotAnonymous {
    async fn authorize(&self, context: &Context<'_>) -> Result<Option<Outcome>, AuthError> {
        Ok(match context.identity {
            UserInformation::Anonymous => Some(Outcome::Deny),
            UserInformation::Authenticated(_) => None,
        })
    }
}
