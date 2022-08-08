use actix_http::body::BoxBody;
use actix_web::{HttpResponse, ResponseError};
use drogue_client::error::ErrorInformation;

#[derive(Clone, Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Forbidden")]
    Forbidden,
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    #[error("Internal: {0}")]
    Internal(String),
    #[error("Resource not found: {0} / {1}")]
    NotFound(String, String),
}

impl ResponseError for AuthError {
    fn error_response(&self) -> HttpResponse<BoxBody> {
        match self {
            Self::Forbidden => HttpResponse::Forbidden().json(ErrorInformation {
                error: "Forbidden".to_string(),
                message: self.to_string(),
            }),
            Self::InvalidRequest(_) => HttpResponse::Forbidden().json(ErrorInformation {
                error: "Forbidden".to_string(),
                message: self.to_string(),
            }),
            Self::Internal(_) => HttpResponse::InternalServerError().json(ErrorInformation {
                error: "Internal".to_string(),
                message: self.to_string(),
            }),
            Self::NotFound(..) => HttpResponse::NotFound().json(ErrorInformation {
                error: "NotFound".to_string(),
                message: self.to_string(),
            }),
        }
    }
}
