//! Stored-procedure boundary for owned, live manual-search attempts.
//!
//! Claims retain the creation procedure's eligibility and policy snapshot. A
//! per-attempt token fences subsequent writes; the outbound correlation remains
//! stable across retries. Plain credentials never implement diagnostic output.

use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use super::search_results::SearchResultIngestInput;
use crate::error::{Result, try_op};

const CLAIM: &str = r"
    SELECT * FROM manual_search_claim_next_v1()
";

const ACTIVE: &str = r"
    SELECT manual_search_active_v1(
        search_request_public_id_input => $1,
        indexer_instance_public_id_input => $2,
        claim_token_input => $3
    )
";

const COMPLETE: &str = r"
    SELECT manual_search_finish_v1(
        search_request_public_id_input => $1,
        indexer_instance_public_id_input => $2,
        claim_token_input => $3,
        error_class_input => $4::error_class,
        canceled_input => $5,
        http_status_input => $6,
        latency_ms_input => $7,
        result_count_input => $8,
        retry_after_seconds_input => $9
    )
";

const INGEST: &str = r"
    SELECT manual_search_ingest_v1(
        search_request_public_id_input => $1,
        indexer_instance_public_id_input => $2,
        source_guid_input => $3,
        details_url_input => $4,
        download_url_input => $5,
        magnet_uri_input => $6,
        title_raw_input => $7,
        size_bytes_input => $8,
        infohash_v1_input => $9,
        infohash_v2_input => $10,
        magnet_hash_input => $11,
        seeders_input => $12,
        leechers_input => $13,
        published_at_input => $14,
        uploader_input => $15,
        observed_at_input => $16,
        attr_keys_input => $17::observation_attr_key[],
        attr_types_input => $18::attr_value_type[],
        attr_value_text_input => $19,
        attr_value_int_input => $20,
        attr_value_bigint_input => $21,
        attr_value_numeric_input => $22::numeric[],
        attr_value_bool_input => $23,
        attr_value_uuid_input => $24,
        claim_token_input => $25,
        details_private_input => $26,
        download_private_input => $27,
        magnet_private_input => $28
    )
";

/// Expected ownership outcomes; database errors are returned separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManualSearchWriteOutcome {
    /// This owner applied the operation.
    Applied,
    /// Cancellation, completion or another attempt superseded this owner.
    OwnershipLost,
}

/// Classified attempt outcome without upstream text or credential-bearing URLs.
#[derive(Debug, Clone, Copy)]
pub enum ManualSearchOutcome {
    /// A recognized response, including a valid empty feed.
    Success,
    /// An existing database error-class label.
    Failure(&'static str),
    /// Cooperative shutdown interrupted the owned execution.
    Canceled,
}

/// Bounded outbound measurements passed to the existing attempt ledger.
#[derive(Debug, Clone, Copy)]
pub struct ManualSearchCompletion {
    /// Classified outcome; raw upstream messages are deliberately not accepted.
    pub outcome: ManualSearchOutcome,
    /// HTTP status when one was received.
    pub http_status: Option<i32>,
    /// Measured elapsed milliseconds.
    pub latency_ms: i32,
    /// Parsed result count on success.
    pub result_count: i32,
    /// Optional upstream retry delay, interpreted by the existing backoff procedure.
    pub retry_after_seconds: Option<i32>,
}

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

/// One atomically owned search attempt, consumed by the production executor.
pub struct ManualSearchRun {
    /// Existing request identity.
    pub search_request_public_id: Uuid,
    /// Selected source identity.
    pub indexer_instance_public_id: Uuid,
    /// Unique attempt ownership; never substitute the stable correlation ID.
    pub claim_token: Uuid,
    /// Stable outbound correlation for this logical fetch across retries.
    pub correlation_id: Uuid,
    /// Existing bounded retry sequence, excluding rate-limit deferrals.
    pub retry_seq: i16,
    /// Query normalized by the creation procedure.
    pub query_text: String,
    /// Configured connection deadline.
    pub connect_timeout_ms: i32,
    /// Configured response deadline.
    pub read_timeout_ms: i32,
    /// Optional explicit route for outbound attempt logging.
    pub routing_policy_public_id: Option<Uuid>,
    /// Configuration errors remain explicit and are completed without network I/O.
    pub connection: std::result::Result<ManualSearchConnection, ManualSearchConfigurationError>,
}

#[derive(FromRow)]
struct ClaimRow {
    search_request_public_id: Uuid,
    indexer_instance_public_id: Uuid,
    claim_token: Uuid,
    correlation_id: Uuid,
    retry_seq: i16,
    query_text: String,
    connect_timeout_ms: i32,
    read_timeout_ms: i32,
    routing_policy_public_id: Option<Uuid>,
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

impl ClaimRow {
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

    fn into_run(self) -> ManualSearchRun {
        let routing = self.resolve_routing();
        let connection = if self.engine != "torznab" {
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
        };
        ManualSearchRun {
            search_request_public_id: self.search_request_public_id,
            indexer_instance_public_id: self.indexer_instance_public_id,
            claim_token: self.claim_token,
            correlation_id: self.correlation_id,
            retry_seq: self.retry_seq,
            query_text: self.query_text,
            connect_timeout_ms: self.connect_timeout_ms,
            read_timeout_ms: self.read_timeout_ms,
            routing_policy_public_id: self.routing_policy_public_id,
            connection,
        }
    }
}

/// Claim one due run; `None` means no executable attempt is currently available.
///
/// # Errors
/// Returns database errors without exposing connection configuration or secrets.
pub async fn claim_next(pool: &PgPool) -> Result<Option<ManualSearchRun>> {
    try_op("manual search claim", || async {
        sqlx::query_as::<_, ClaimRow>(CLAIM)
            .fetch_optional(pool)
            .await
            .map(|row| row.map(ClaimRow::into_run))
    })
    .await
}

/// Observe whether this attempt still owns the live request.
///
/// # Errors
/// Returns database errors; cancellation is the expected `false` outcome.
pub async fn is_active(pool: &PgPool, run: &ManualSearchRun) -> Result<bool> {
    try_op("manual search ownership", || {
        sqlx::query_scalar(ACTIVE)
            .bind(run.search_request_public_id)
            .bind(run.indexer_instance_public_id)
            .bind(run.claim_token)
            .fetch_one(pool)
    })
    .await
}

/// Settle an owned attempt atomically with its outbound log and existing backoff.
///
/// # Errors
/// Returns database errors without exposing credentials. Stale ownership is explicit.
pub async fn complete(
    pool: &PgPool,
    run: &ManualSearchRun,
    completion: ManualSearchCompletion,
) -> Result<ManualSearchWriteOutcome> {
    let error_class = match completion.outcome {
        ManualSearchOutcome::Success => None,
        ManualSearchOutcome::Failure(class) => Some(class),
        ManualSearchOutcome::Canceled => Some("unknown"),
    };
    let applied: bool = try_op("manual search completion", || {
        sqlx::query_scalar(COMPLETE)
            .bind(run.search_request_public_id)
            .bind(run.indexer_instance_public_id)
            .bind(run.claim_token)
            .bind(error_class)
            .bind(matches!(completion.outcome, ManualSearchOutcome::Canceled))
            .bind(completion.http_status)
            .bind(completion.latency_ms)
            .bind(completion.result_count)
            .bind(completion.retry_after_seconds)
            .fetch_one(pool)
    })
    .await?;
    Ok(if applied {
        ManualSearchWriteOutcome::Applied
    } else {
        ManualSearchWriteOutcome::OwnershipLost
    })
}

/// Visibility of upstream URLs; private links remain available for acquisition.
#[derive(Clone, Copy, Default)]
pub struct ManualSearchLinkPrivacy {
    /// The details URL contains authentication material.
    pub details: bool,
    /// The download URL contains authentication material.
    pub download: bool,
    /// The magnet URI contains authentication material.
    pub magnet: bool,
}

/// Ingest one observation only while its attempt still owns the request.
///
/// # Errors
/// Returns database errors. Cancellation and stale ownership perform no writes.
pub async fn ingest(
    pool: &PgPool,
    run: &ManualSearchRun,
    input: &SearchResultIngestInput<'_>,
    privacy: ManualSearchLinkPrivacy,
) -> Result<ManualSearchWriteOutcome> {
    let applied: bool = try_op("owned manual search ingest", || {
        sqlx::query_scalar(INGEST)
            .bind(input.search_request_public_id)
            .bind(input.indexer_instance_public_id)
            .bind(input.source_guid)
            .bind(input.details_url)
            .bind(input.download_url)
            .bind(input.magnet_uri)
            .bind(input.title_raw)
            .bind(input.size_bytes)
            .bind(input.infohash_v1)
            .bind(input.infohash_v2)
            .bind(input.magnet_hash)
            .bind(input.seeders)
            .bind(input.leechers)
            .bind(input.published_at)
            .bind(input.uploader)
            .bind(input.observed_at)
            .bind(input.attr_keys)
            .bind(input.attr_types)
            .bind(input.attr_value_text)
            .bind(input.attr_value_int)
            .bind(input.attr_value_bigint)
            .bind(input.attr_value_numeric)
            .bind(input.attr_value_bool)
            .bind(input.attr_value_uuid)
            .bind(run.claim_token)
            .bind(privacy.details)
            .bind(privacy.download)
            .bind(privacy.magnet)
            .fetch_one(pool)
    })
    .await?;
    Ok(if applied {
        ManualSearchWriteOutcome::Applied
    } else {
        ManualSearchWriteOutcome::OwnershipLost
    })
}

#[cfg(test)]
mod tests;
