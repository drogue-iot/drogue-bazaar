use std::sync::Arc;
use actix_cors::Cors;
use serde::Deserialize;
use crate::actix::http::CorsBuilder;

#[derive(Clone, Debug, Default, Deserialize)]
pub struct CorsConfig {
    // default for bool is false
    #[serde(default)]
    pub allow_any_origin: bool,

    #[serde(default)]
    pub allow_origin_url: Option<String>,
}

impl From<CorsConfig> for Cors {
    fn from(cfg: CorsConfig) -> Cors {
        let mut cors = Cors::default()
            .allowed_headers(vec![
                http::header::AUTHORIZATION,
                http::header::CONTENT_TYPE,
            ])
            .max_age(3600);

        if cfg.allow_any_origin {
            cors = cors.allow_any_origin();
        } else if let Some(origin) = &cfg.allow_origin_url {
            cors = cors.allowed_origin(origin.as_str());
        }
        cors
    }
}

impl From<CorsConfig> for CorsBuilder {
    fn from(config: CorsConfig) -> Self {
        if config.allow_any_origin {
            CorsBuilder::Permissive
        } else if config.allow_origin_url.is_some() {
            CorsBuilder::Custom(config.into())
        } else {
            CorsBuilder::Disabled
        }
    }
}