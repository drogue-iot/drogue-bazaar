#[cfg(feature = "actix")]
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

#[cfg(feature = "actix")]
impl actix_web::ResponseError for AuthError {
    fn error_response(&self) -> actix_web::HttpResponse<actix_http::body::BoxBody> {
        match self {
            Self::Forbidden => actix_web::HttpResponse::Forbidden().json(ErrorInformation {
                error: "Forbidden".to_string(),
                message: self.to_string(),
            }),
            Self::InvalidRequest(_) => {
                actix_web::HttpResponse::Forbidden().json(ErrorInformation {
                    error: "Forbidden".to_string(),
                    message: self.to_string(),
                })
            }
            Self::Internal(_) => {
                actix_web::HttpResponse::InternalServerError().json(ErrorInformation {
                    error: "Internal".to_string(),
                    message: self.to_string(),
                })
            }
            Self::NotFound(..) => actix_web::HttpResponse::NotFound().json(ErrorInformation {
                error: "NotFound".to_string(),
                message: self.to_string(),
            }),
        }
    }
}
