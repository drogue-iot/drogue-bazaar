use crate::core::config::CommaSeparatedVec;
use actix_cors::Cors;
use http::header::{HeaderName, InvalidHeaderName};
use http::method::InvalidMethod;
use http::Method;
use serde::Deserialize;
use std::str::FromStr;
use std::time::Duration;

#[derive(Clone, Debug, Default, Deserialize)]
pub struct CorsSettings {
    #[serde(default)]
    pub allowed_origin_urls: Option<CommaSeparatedVec>,

    #[serde(default)]
    pub allowed_methods: Option<CommaSeparatedVec>,

    #[serde(default)]
    pub allowed_headers: Option<CommaSeparatedVec>,

    #[serde(default)]
    pub allow_any_method: bool,

    #[serde(default)]
    pub allow_any_header: bool,

    #[serde(default)]
    pub allow_any_origin: bool,

    #[serde(default)]
    pub expose_headers: Option<CommaSeparatedVec>,

    #[serde(default)]
    #[serde(with = "humantime_serde")]
    pub max_age: Option<Duration>,

    #[serde(default)]
    pub disable_preflight: bool,

    #[serde(default)]
    pub send_wildcard: bool,

    #[serde(default)]
    pub disable_vary_header: bool,

    #[serde(default)]
    pub expose_any_header: bool,

    #[serde(default)]
    pub supports_credentials: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "mode")]
pub enum CorsConfig {
    Disabled,
    Permissive(CorsSettings),
    Custom(CorsSettings),
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self::Disabled
    }
}

impl CorsConfig {
    /// Create a default "permissive" configuration.
    ///
    /// This creates a [`Cors::permissive()`] based instance, with no customizations.
    pub fn permissive() -> Self {
        Self::Permissive(Default::default())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CorsConfigError {
    #[error("Invalid HTTP header name: {0}")]
    InvalidHeaderName(#[from] InvalidHeaderName),
    #[error("Invalid HTTP method: {0}")]
    InvalidMethod(#[from] InvalidMethod),
}

impl CorsSettings {
    pub fn apply(self, mut cors: Cors) -> Result<Cors, CorsConfigError> {
        if let Some(max_age) = self.max_age.map(|age| age.as_secs() as usize) {
            cors = cors.max_age(max_age);
        }

        if let Some(headers) = self.allowed_headers()? {
            cors = cors.allowed_headers(headers);
        }

        if let Some(origin) = &self.allowed_origin_urls {
            for url in &origin.0 {
                cors = cors.allowed_origin(url.as_str());
            }
        }

        if let Some(methods) = self.allowed_methods()? {
            cors = cors.allowed_methods(methods);
        }

        if self.send_wildcard {
            cors = cors.send_wildcard()
        }

        if self.disable_preflight {
            cors = cors.disable_preflight();
        }

        if self.disable_vary_header {
            cors = cors.disable_vary_header();
        }

        if self.allow_any_method {
            cors = cors.allow_any_method();
        }

        if self.allow_any_header {
            cors = cors.allow_any_header();
        }

        if self.allow_any_origin {
            cors = cors.allow_any_origin();
        }

        if self.supports_credentials {
            cors = cors.supports_credentials();
        }

        if let Some(headers) = self.expose_headers()? {
            cors = cors.expose_headers(headers);
        }

        if self.expose_any_header {
            cors = cors.expose_any_header();
        }

        Ok(cors)
    }

    /// Evaluate the allowed headers.
    fn allowed_headers(&self) -> Result<Option<Vec<HeaderName>>, InvalidHeaderName> {
        Self::convert_headers(&self.allowed_headers)
    }

    /// Evaluate the expose headers.
    fn expose_headers(&self) -> Result<Option<Vec<HeaderName>>, InvalidHeaderName> {
        Self::convert_headers(&self.expose_headers)
    }

    /// Convert headers from string to [`HeaderName].
    ///
    /// Failing the operation if one of the conversions fails.
    fn convert_headers(
        headers: &Option<CommaSeparatedVec>,
    ) -> Result<Option<Vec<HeaderName>>, InvalidHeaderName> {
        Ok(headers
            .as_ref()
            .map(|csv| &csv.0)
            .map(|headers| {
                headers
                    .into_iter()
                    .map(|h| HeaderName::from_str(&h))
                    .collect::<Result<_, _>>()
            })
            .transpose()?)
    }

    fn allowed_methods(&self) -> Result<Option<Vec<Method>>, InvalidMethod> {
        Ok(self
            .allowed_methods
            .as_ref()
            .map(|csv| &csv.0)
            .map(|methods| {
                methods
                    .into_iter()
                    .map(|m| Method::from_str(&m))
                    .collect::<Result<_, _>>()
            })
            .transpose()?)
    }
}

impl TryFrom<CorsConfig> for Option<Cors> {
    type Error = CorsConfigError;

    fn try_from(cfg: CorsConfig) -> Result<Option<Cors>, CorsConfigError> {
        Ok(match cfg {
            CorsConfig::Disabled => None,
            CorsConfig::Permissive(settings) => Some(settings.apply(Cors::permissive())?),
            CorsConfig::Custom(settings) => Some(settings.apply(Cors::default())?),
        })
    }
}

/// Testing stuff.
///
/// Unfortunately `Cors` doesn't allow to be inspected. This means, that we have a hard time
/// to figure out if the configuration produces the expected result. In some cases it is possible
/// to use the debug representation. But in other cases, the data contains `HashSet`s, which don't
/// have a stable order.
#[cfg(test)]
mod test {
    use super::*;
    use crate::core::config::ConfigFromEnv;
    use config::Environment;
    use std::collections::HashMap;

    fn make_cors(input: &[(&str, &str)]) -> Result<Option<Cors>, CorsConfigError> {
        #[derive(Clone, Debug, Deserialize)]
        struct Test {
            cors: CorsConfig,
        }

        let mut env = HashMap::<String, String>::new();
        for e in input {
            env.insert(e.0.to_string(), e.1.to_string());
        }

        let cfg =
            <Test as ConfigFromEnv>::from(Environment::default().prefix("HTTP").source(Some(env)))
                .unwrap();

        cfg.cors.try_into()
    }

    #[test]
    fn test_config_disabled() {
        let cors = make_cors(&[("HTTP__CORS__MODE", "disabled")]).unwrap();

        assert!(cors.is_none());
    }

    #[test]
    fn test_config_permissive() {
        let actual = make_cors(&[("HTTP__CORS__MODE", "permissive")])
            .unwrap()
            .unwrap();

        let expected = Cors::permissive();

        assert_eq!(format!("{actual:?}"), format!("{expected:?}"));
    }

    #[test]
    fn test_config_custom() {
        let actual = make_cors(&[("HTTP__CORS__MODE", "custom")])
            .unwrap()
            .unwrap();

        let expected = Cors::default();

        assert_eq!(format!("{actual:?}"), format!("{expected:?}"));
    }

    #[test]
    fn test_config_permissive_with() {
        let actual = make_cors(&[
            ("HTTP__CORS__MODE", "permissive"),
            ("HTTP__CORS__MAX_AGE", "1h"),
        ])
        .unwrap()
        .unwrap();

        let expected = Cors::permissive().max_age(3600);

        assert_eq!(format!("{actual:?}"), format!("{expected:?}"));
    }

    #[test]
    fn test_config_custom_with() {
        let actual = make_cors(&[
            ("HTTP__CORS__MODE", "custom"),
            ("HTTP__CORS__MAX_AGE", "1h"),
            ("HTTP__CORS__ALLOWED_METHODS", "GET,POST"),
            (
                "HTTP__CORS__ALLOWED_ORIGIN_URLS",
                "https://foo.bar,https://bar.baz/*",
            ),
        ])
        .unwrap()
        .unwrap();

        let debug = format!("{actual:?}");

        assert!(debug.contains("GET"));
        assert!(debug.contains("POST"));

        assert!(debug.contains("https://foo.bar"));
        assert!(debug.contains("https://bar.baz/*"));
    }
}
