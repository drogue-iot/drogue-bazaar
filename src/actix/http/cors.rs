use actix_cors::Cors;
use http::Method;
use serde::Deserialize;
use std::str::FromStr;

#[derive(Clone, Debug, Default, Deserialize)]
pub struct CorsConfig {
    // default for bool is false
    #[serde(default)]
    pub allow_any_origin: bool,

    #[serde(default)]
    pub allow_origin_url: Option<Vec<String>>,

    #[serde(default)]
    pub allowed_methods: Option<Vec<String>>,
}

impl CorsConfig {
    pub fn set_allowed_methods(&mut self, methods: Vec<&str>) -> &Self {
        let methods: Vec<String> = methods.into_iter().map(|m| m.into()).collect();
        self.allowed_methods = Some(methods);
        self
    }

    pub fn set_allowed_urls(&mut self, urls: Vec<&str>) -> &Self {
        let url: Vec<String> = urls.into_iter().map(|m| m.into()).collect();
        self.allow_origin_url = Some(url);
        self
    }
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
            for url in origin {
                cors = cors.allowed_origin(url.as_str());
            }
        }

        if let Some(methods) = cfg.allowed_methods {
            let methods: Vec<Method> = methods
                .into_iter()
                .filter_map(|m| Method::from_str(m.as_str()).ok())
                .collect();
            cors = cors.allowed_methods(methods);
        }
        cors
    }
}
