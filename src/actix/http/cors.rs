use crate::core::config::CommaSeparatedVec;
use actix_cors::Cors;
use http::Method;
use serde::Deserialize;
use std::str::FromStr;

#[derive(Clone, Debug, Default, Deserialize)]
pub struct CorsConfig {
    #[serde(default)]
    pub mode: CorsMode,

    #[serde(default)]
    pub allow_origin_urls: Option<CommaSeparatedVec>,

    #[serde(default)]
    pub allowed_methods: Option<CommaSeparatedVec>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CorsMode {
    Permissive,
    Disabled,
    Manual,
}

impl Default for CorsMode {
    fn default() -> Self {
        CorsMode::Disabled
    }
}

impl CorsConfig {
    pub fn permissive() -> CorsConfig {
        CorsConfig {
            mode: CorsMode::Permissive,
            ..Default::default()
        }
    }

    pub fn disabled() -> CorsConfig {
        CorsConfig {
            mode: CorsMode::Disabled,
            ..Default::default()
        }
    }

    pub fn set_allowed_methods(mut self, methods: Vec<&str>) -> Self {
        let methods: Vec<String> = methods.into_iter().map(|m| m.into()).collect();
        self.allowed_methods = Some(methods.into());
        self
    }

    pub fn set_allowed_urls(mut self, urls: Vec<&str>) -> Self {
        let url: Vec<String> = urls.into_iter().map(|m| m.into()).collect();
        self.allow_origin_urls = Some(url.into());
        self
    }
}

impl From<CorsConfig> for Option<Cors> {
    fn from(cfg: CorsConfig) -> Option<Cors> {
        match cfg.mode {
            CorsMode::Disabled => None,
            CorsMode::Permissive => Some(Cors::permissive()),
            CorsMode::Manual => {
                let mut cors = Cors::default()
                    .allowed_headers(vec![
                        http::header::AUTHORIZATION,
                        http::header::CONTENT_TYPE,
                    ])
                    .max_age(cfg.max_age.as_secs() as usize);

                if let Some(origin) = &cfg.allow_origin_urls {
                    for url in &origin.0 {
                        cors = cors.allowed_origin(url.as_str());
                    }
                }

                if let Some(methods) = cfg.allowed_methods {
                    let methods: Vec<Method> = methods
                        .0
                        .into_iter()
                        .filter_map(|m| Method::from_str(m.as_str()).ok())
                        .collect();
                    cors = cors.allowed_methods(methods);
                }
                Some(cors)
            }
        }
    }
}
