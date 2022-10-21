use actix_cors::Cors;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub enum Method {
    GET,
    POST,
    PUT,
    DELETE
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct CorsConfig {
    // default for bool is false
    #[serde(default)]
    pub allow_any_origin: bool,

    #[serde(default)]
    pub allow_origin_url: Option<Vec<String>>,

    #[serde(default)]
    pub allowed_methods: Option<Vec<Method>>
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
            let methods: Vec<&str> = methods.into_iter().map(|m| m.into()).collect();
            cors = cors.allowed_methods(methods);
        }
        cors
    }
}

impl From<Method> for &str {
    fn from(method: Method) -> Self {
        match method {
            Method::GET => "GET",
            Method::POST => "POST",
            Method::PUT => "PUT",
            Method::DELETE => "DELETE",
        }
    }
}