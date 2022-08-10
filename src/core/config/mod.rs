mod csv;

pub use csv::*;

use serde::Deserialize;
use std::collections::HashMap;

/// A default setup to extract configuration from environment variables.
///
/// The default setup uses `__` (double underscore) as a delimiter.
///
/// ```
/// use drogue_bazaar::core::config::ConfigFromEnv;
///
/// #[derive(serde::Deserialize)]
/// struct Config {
///     sub: SubConfig,
///     my_str: String,
/// }
///
/// #[derive(serde::Deserialize)]
/// struct SubConfig {
///     my_str: String,
///     #[serde(default)]
///     my_bool: bool,
///     #[serde(default)]
///     my_opt_int: Option<u32>,
/// }
///
/// fn run() -> anyhow::Result<()> {
///     /*
///     Assume the following env-vars are set:
///         MY_STR = abc
///         SUB__MY_STR = def
///     */
///     let config = Config::from_env()?;
///
///     /* The struct would be: {
///         my_str: "abc",
///         sub: {
///             my_str: "def",
///             my_bool: false,
///             my_opt_int: None,
///         }
///     } */
///
///     Ok(())
/// }
/// ```
pub trait ConfigFromEnv<'de>: Sized + Deserialize<'de> {
    /// Get a configuration from the env-vars.
    fn from_env() -> Result<Self, config::ConfigError> {
        Self::from(config::Environment::default())
    }

    /// Get a configuration from the env-vars, prefixing all with the provided prefix **plus**
    /// the separator.
    fn from_env_prefix<S: AsRef<str>>(prefix: S) -> Result<Self, config::ConfigError> {
        Self::from(config::Environment::with_prefix(prefix.as_ref()))
    }

    fn from(env: config::Environment) -> Result<Self, config::ConfigError>;

    fn from_set<K, V>(set: HashMap<K, V>) -> Result<Self, config::ConfigError>
    where
        K: Into<String>,
        V: Into<String>,
    {
        let set = set.into_iter().map(|(k, v)| (k.into(), v.into())).collect();
        Self::from(config::Environment::default().source(Some(set)))
    }
}

impl<'de, T: Deserialize<'de> + Sized> ConfigFromEnv<'de> for T {
    fn from(env: config::Environment) -> Result<T, config::ConfigError> {
        let env = env.try_parsing(true).separator("__");

        let cfg = config::Config::builder().add_source(env);
        cfg.build()?.try_deserialize()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use config::Environment;
    use serde::Deserialize;
    use std::collections::HashMap;

    #[test]
    fn test_prefix() {
        #[derive(Debug, Deserialize)]
        struct Foo {
            pub bar: String,
            pub r#bool: bool,
        }

        let mut env = HashMap::<String, String>::new();
        env.insert("FOO__BAR".into(), "baz".into());
        env.insert("FOO__BOOL".into(), "true".into());

        let foo = <Foo as ConfigFromEnv>::from(Environment::with_prefix("FOO").source(Some(env)))
            .unwrap();
        assert_eq!(foo.bar, "baz");
        assert_eq!(foo.r#bool, true);
    }

    #[test]
    fn test_nested() {
        #[derive(Debug, Deserialize)]
        struct Foo {
            #[serde(default)]
            pub bar: Option<Bar>,
        }
        #[derive(Debug, Deserialize)]
        struct Bar {
            pub baz: Baz,
        }
        #[derive(Debug, Deserialize)]
        struct Baz {
            pub value: String,
        }

        let mut env = HashMap::<String, String>::new();
        env.insert("FOO__BAR__BAZ__VALUE".into(), "s1".into());

        let foo =
            <Foo as ConfigFromEnv>::from(Environment::default().prefix("FOO").source(Some(env)))
                .unwrap();

        assert_eq!(foo.bar.unwrap().baz.value, "s1");
    }
}
