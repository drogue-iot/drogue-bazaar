use crate::core::tls::TlsAuthConfig;
use actix_http::{Request, Response};
use actix_service::{IntoServiceFactory, ServiceFactory};
use actix_web::{body::MessageBody, dev::AppConfig, Error, HttpServer};
use std::{fmt, path::Path};
use tokio::io;

/// Bind HTTP server to HTTP or HTTPS port, using an enabled TLS implementation.
pub fn bind_http<F, I, S, B, K, C>(
    main: HttpServer<F, I, S, B>,
    bind_addr: String,
    tls_auth_config: Option<TlsAuthConfig>,
    key_file: Option<K>,
    cert_bundle_file: Option<C>,
) -> io::Result<HttpServer<F, I, S, B>>
where
    F: Fn() -> I + Send + Clone + 'static,
    I: IntoServiceFactory<S, Request>,
    S: ServiceFactory<Request, Config = AppConfig> + 'static,
    S::Error: Into<Error> + 'static,
    S::InitError: fmt::Debug,
    S::Response: Into<Response<B>> + 'static,
    B: MessageBody + 'static,
    K: AsRef<Path>,
    C: AsRef<Path>,
{
    match (tls_auth_config, key_file, cert_bundle_file) {
        #[allow(unused_variables)]
        (Some(tls_auth_config), Some(key), Some(cert)) => {
            #[cfg(feature = "openssl")]
            if cfg!(feature = "openssl") {
                return bind_http_openssl(main, tls_auth_config, bind_addr, key, cert);
            }
            panic!("TLS is required, but no TLS implementation enabled")
        }
        (None, None, None) => main.bind(bind_addr),
        (Some(_), _, _) => {
            panic!("Wrong TLS configuration: TLS enabled, but key or cert is missing")
        }
        (None, Some(_), _) | (None, _, Some(_)) => {
            // the TLS configuration must be consistent, to prevent configuration errors.
            panic!("Wrong TLS configuration: key or cert specified, but TLS is disabled")
        }
    }
}

#[cfg(feature = "openssl")]
fn bind_http_openssl<F, I, S, B, K, C>(
    main: HttpServer<F, I, S, B>,
    tls_auth_config: TlsAuthConfig,
    bind_addr: String,
    key_file: K,
    cert_bundle_file: C,
) -> io::Result<HttpServer<F, I, S, B>>
where
    F: Fn() -> I + Send + Clone + 'static,
    I: IntoServiceFactory<S, Request>,
    S: ServiceFactory<Request, Config = AppConfig> + 'static,
    S::Error: Into<Error> + 'static,
    S::InitError: fmt::Debug,
    S::Response: Into<Response<B>> + 'static,
    B: MessageBody + 'static,
    K: AsRef<Path>,
    C: AsRef<Path>,
{
    use crate::core::tls::TlsMode;
    use openssl::ssl;

    let method = ssl::SslMethod::tls_server();
    let mut builder = ssl::SslAcceptor::mozilla_intermediate_v5(method)?;
    builder.set_private_key_file(key_file, ssl::SslFiletype::PEM)?;
    builder.set_certificate_chain_file(cert_bundle_file)?;

    if let TlsMode::Client = tls_auth_config.mode {
        // we ask for client certificates, but don't enforce them
        builder.set_verify_callback(ssl::SslVerifyMode::PEER, |_, ctx| {
            log::debug!(
                "Accepting client certificates: {:?}",
                ctx.current_cert()
                    .map(|cert| format!("{:?}", cert.subject_name()))
                    .unwrap_or_else(|| "<unknown>".into())
            );
            true
        });
    }

    if let Some(psk) = tls_auth_config.psk {
        builder.set_psk_server_callback(move |ssl, identity, secret_mut| {
            match psk(ssl, identity, secret_mut) {
                Ok(len) => Ok(len),
                Err(e) => {
                    log::debug!("Error during TLS-PSK handshake: {:?}", e);
                    Ok(0)
                }
            }
        });
    }

    Ok(main
        .bind_openssl(bind_addr, builder)?
        .tls_handshake_timeout(std::time::Duration::from_secs(10)))
}
