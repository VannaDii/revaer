//! Shared private connection settings for concrete Torznab search and RSS callers.

use sqlx::FromRow;

/// A configuration failure to settle without issuing an upstream request.
#[derive(Debug, Clone, Copy)]
pub enum ManualSearchConfigurationError {
    /// The selected engine is not the outbound Torznab adapter.
    UnsupportedEngine,
    /// The endpoint has not been configured.
    MissingEndpoint,
    /// The credential binding is absent, revoked, or cannot be decrypted.
    CredentialUnavailable,
    /// The configured route is incomplete or unsupported by this executor.
    InvalidRoute,
}

impl std::fmt::Display for ManualSearchConfigurationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidRoute => "configure a supported, complete outbound route",
            Self::UnsupportedEngine => "manual search requires a Torznab indexer",
            Self::MissingEndpoint => "configure the Torznab API URL",
            Self::CredentialUnavailable => {
                "configure an available Torznab API key with the active encryption key"
            }
        })
    }
}

impl std::error::Error for ManualSearchConfigurationError {}

/// Effective outbound routing; credentials deliberately have no diagnostic representation.
pub enum ManualSearchRouting {
    /// Use the direct route with the operator's TLS verification setting.
    Direct {
        /// Verify the upstream TLS certificate.
        verify_tls: bool,
    },
    /// Use the configured HTTP or SOCKS proxy, without a direct fallback.
    Proxy {
        /// Verify TLS certificates for this route.
        verify_tls: bool,
        /// Resolved proxy connection and optional authentication.
        proxy: ManualSearchProxy,
    },
}

/// Proxy parameters resolved from normal routing fields and encrypted bindings.
pub struct ManualSearchProxy {
    /// Existing proxy protocol: HTTP, HTTPS or SOCKS with remote DNS.
    pub scheme: &'static str,
    /// Configured proxy hostname or IP address.
    pub host: String,
    /// Validated proxy port.
    pub port: u16,
    /// Optional username/password pair. Absence means an unauthenticated proxy.
    pub authentication: Option<(String, String)>,
}

/// Resolved upstream connection; intentionally has no `Debug` implementation.
pub struct ManualSearchConnection {
    /// Operator-configured endpoint, without any diagnostic formatting.
    pub endpoint: String,
    /// Decrypted key for this owned execution only.
    pub api_key: String,
    /// Effective route, including the implicit direct route.
    pub routing: ManualSearchRouting,
}

#[derive(FromRow)]
pub(crate) struct ConnectionRow {
    routing_mode: String,
    engine: String,
    endpoint: Option<String>,
    api_key: Option<String>,
    credential_error: Option<String>,
    verify_tls: bool,
    proxy_host: Option<String>,
    proxy_port: Option<i32>,
    proxy_use_tls: bool,
    proxy_username: Option<String>,
    proxy_password: Option<String>,
}

impl ConnectionRow {
    fn resolve_routing(
        &self,
    ) -> std::result::Result<ManualSearchRouting, ManualSearchConfigurationError> {
        use ManualSearchConfigurationError::{CredentialUnavailable, InvalidRoute};
        let scheme = match self.routing_mode.as_str() {
            "direct" => {
                return Ok(ManualSearchRouting::Direct {
                    verify_tls: self.verify_tls,
                });
            }
            "http_proxy" => {
                if self.proxy_use_tls {
                    "https"
                } else {
                    "http"
                }
            }
            "socks_proxy" => "socks5h",
            _ => return Err(InvalidRoute),
        };
        let host = self
            .proxy_host
            .as_ref()
            .filter(|host| !host.is_empty())
            .ok_or(InvalidRoute)?
            .clone();
        let port = self
            .proxy_port
            .and_then(|port| u16::try_from(port).ok())
            .filter(|port| *port != 0)
            .ok_or(InvalidRoute)?;
        let authentication = match (&self.proxy_username, &self.proxy_password) {
            (Some(user), Some(secret)) => Some((user.clone(), secret.clone())),
            (None, None) => None,
            _ => return Err(CredentialUnavailable),
        };
        Ok(ManualSearchRouting::Proxy {
            verify_tls: self.verify_tls,
            proxy: ManualSearchProxy {
                scheme,
                host,
                port,
                authentication,
            },
        })
    }

    pub(crate) fn resolve(self) -> Result<ManualSearchConnection, ManualSearchConfigurationError> {
        let routing = self.resolve_routing();
        if self.engine != "torznab" {
            Err(ManualSearchConfigurationError::UnsupportedEngine)
        } else if self.credential_error.is_some() {
            Err(ManualSearchConfigurationError::CredentialUnavailable)
        } else if let Some(endpoint) = self.endpoint {
            self.api_key.map_or_else(
                || Err(ManualSearchConfigurationError::CredentialUnavailable),
                |api_key| {
                    routing.map(|routing| ManualSearchConnection {
                        endpoint,
                        api_key,
                        routing,
                    })
                },
            )
        } else {
            Err(ManualSearchConfigurationError::MissingEndpoint)
        }
    }
}
