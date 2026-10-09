//! Prometheus-backed metrics registry and snapshot helpers.
//!
//! # Design
//! - Encapsulates collector registration to keep the public API small.
//! - Exposes a minimal set of counters/gauges relevant to Revaer services.

use std::convert::TryFrom;
use std::time::Duration;

use crate::error::{Result, TelemetryError};
use prometheus::core::Collector;
use prometheus::{
    Encoder, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, IntGauge, Opts, Registry,
    TextEncoder,
};
use serde::Serialize;

/// Prometheus-backed metrics registry shared across services.
#[derive(Clone)]
pub struct Metrics {
    inner: std::sync::Arc<MetricsInner>,
}

struct MetricsInner {
    registry: Registry,
    http_requests_total: IntCounterVec,
    events_emitted_total: IntCounterVec,
    fsops_steps_total: IntCounterVec,
    active_torrents: IntGauge,
    queue_depth: IntGauge,
    engine_bytes_in: IntGauge,
    engine_bytes_out: IntGauge,
    config_watch_latency_ms: IntGauge,
    config_apply_latency_ms: IntGauge,
    config_update_failures_total: IntCounter,
    config_watch_slow_total: IntCounter,
    guardrail_violations_total: IntCounter,
    rate_limit_throttled_total: IntCounter,
    torznab_invalid_requests_total: IntCounterVec,
    indexer_search_requests_total: IntCounterVec,
    indexer_job_outcomes_total: IntCounterVec,
    indexer_operations_total: IntCounterVec,
    indexer_operation_latency_ms: HistogramVec,
    media_discovery_candidates_total: IntCounterVec,
    media_jobs_queued_total: IntCounterVec,
    media_capability_refresh_total: IntCounterVec,
    media_job_phases_total: IntCounterVec,
    media_job_operations_total: IntCounterVec,
    media_job_verification_checks_total: IntCounterVec,
    media_job_failures_total: IntCounterVec,
    media_job_outcomes_total: IntCounterVec,
    media_job_duration_ms: HistogramVec,
    media_workspace_cleanup_total: IntCounterVec,
    media_retention_runs_total: IntCounterVec,
    media_retention_rows_total: IntCounterVec,
}

struct MetricsCollectors {
    http_requests_total: IntCounterVec,
    events_emitted_total: IntCounterVec,
    fsops_steps_total: IntCounterVec,
    active_torrents: IntGauge,
    queue_depth: IntGauge,
    engine_bytes_in: IntGauge,
    engine_bytes_out: IntGauge,
    config_watch_latency_ms: IntGauge,
    config_apply_latency_ms: IntGauge,
    config_update_failures_total: IntCounter,
    config_watch_slow_total: IntCounter,
    guardrail_violations_total: IntCounter,
    rate_limit_throttled_total: IntCounter,
    torznab_invalid_requests_total: IntCounterVec,
    indexer_search_requests_total: IntCounterVec,
    indexer_job_outcomes_total: IntCounterVec,
    indexer_operations_total: IntCounterVec,
    indexer_operation_latency_ms: HistogramVec,
    media_discovery_candidates_total: IntCounterVec,
    media_jobs_queued_total: IntCounterVec,
    media_capability_refresh_total: IntCounterVec,
    media_job_phases_total: IntCounterVec,
    media_job_operations_total: IntCounterVec,
    media_job_verification_checks_total: IntCounterVec,
    media_job_failures_total: IntCounterVec,
    media_job_outcomes_total: IntCounterVec,
    media_job_duration_ms: HistogramVec,
    media_workspace_cleanup_total: IntCounterVec,
    media_retention_runs_total: IntCounterVec,
    media_retention_rows_total: IntCounterVec,
}

struct MediaMetricsCollectors {
    discovery_candidates_total: IntCounterVec,
    jobs_queued_total: IntCounterVec,
    capability_refresh_total: IntCounterVec,
    job_phases_total: IntCounterVec,
    job_operations_total: IntCounterVec,
    job_verification_checks_total: IntCounterVec,
    job_failures_total: IntCounterVec,
    job_outcomes_total: IntCounterVec,
    job_duration_ms: HistogramVec,
    workspace_cleanup_total: IntCounterVec,
    retention_runs_total: IntCounterVec,
    retention_rows_total: IntCounterVec,
}

/// Snapshot of selected gauges and counters for health reporting.
#[derive(Debug, Clone, Serialize)]
pub struct MetricsSnapshot {
    /// Current number of active torrents.
    pub active_torrents: i64,
    /// Current queue depth for pending torrents.
    pub queue_depth: i64,
    /// Latest latency (ms) when watching for configuration changes.
    pub config_watch_latency_ms: i64,
    /// Latest latency (ms) when applying configuration changes.
    pub config_apply_latency_ms: i64,
    /// Total count of configuration update failures observed.
    pub config_update_failures_total: u64,
    /// Total count of slow configuration watch intervals observed.
    pub config_watch_slow_total: u64,
    /// Total guardrail violations recorded.
    pub guardrail_violations_total: u64,
    /// Total requests throttled by API rate limiting.
    pub rate_limit_throttled_total: u64,
}

impl Metrics {
    /// Construct a new metrics registry with the standard collectors registered.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the Prometheus collectors cannot be
    /// registered.
    pub fn new() -> Result<Self> {
        let inner = MetricsInner::new(Registry::new())?;
        Ok(Self {
            inner: std::sync::Arc::new(inner),
        })
    }

    /// Increment the HTTP request counter for the given route and status code.
    pub fn inc_http_request(&self, route: &str, status: u16) {
        self.inner
            .http_requests_total
            .with_label_values(&[route, &status.to_string()])
            .inc();
    }

    /// Increment the emitted event counter for the specific event type.
    pub fn inc_event(&self, event_type: &str) {
        self.inner
            .events_emitted_total
            .with_label_values(&[event_type])
            .inc();
    }

    /// Increment the filesystem post-processing step counter.
    pub fn inc_fsops_step(&self, step: &str, status: &str) {
        self.inner
            .fsops_steps_total
            .with_label_values(&[step, status])
            .inc();
    }

    /// Set the active torrent gauge.
    pub fn set_active_torrents(&self, count: i64) {
        self.inner.active_torrents.set(count);
    }

    /// Set the queue depth gauge.
    pub fn set_queue_depth(&self, depth: i64) {
        self.inner.queue_depth.set(depth);
    }

    /// Record inbound bytes observed by the engine.
    pub fn set_engine_bytes_in(&self, value: i64) {
        self.inner.engine_bytes_in.set(value);
    }

    /// Record outbound bytes observed by the engine.
    pub fn set_engine_bytes_out(&self, value: i64) {
        self.inner.engine_bytes_out.set(value);
    }

    /// Record the observed latency while waiting for configuration updates.
    pub fn observe_config_watch_latency(&self, duration: Duration) {
        self.inner
            .config_watch_latency_ms
            .set(Self::duration_to_ms(duration));
    }

    /// Record the observed latency for applying configuration updates.
    pub fn observe_config_apply_latency(&self, duration: Duration) {
        self.inner
            .config_apply_latency_ms
            .set(Self::duration_to_ms(duration));
    }

    /// Increment the configuration update failure counter.
    pub fn inc_config_update_failure(&self) {
        self.inner.config_update_failures_total.inc();
    }

    /// Increment the counter tracking slow configuration applications.
    pub fn inc_config_watch_slow(&self) {
        self.inner.config_watch_slow_total.inc();
    }

    /// Increment the guardrail violation counter (e.g. setup loopback enforcement).
    pub fn inc_guardrail_violation(&self) {
        self.inner.guardrail_violations_total.inc();
    }

    /// Increment the API rate limiter throttle counter.
    pub fn inc_rate_limit_throttled(&self) {
        self.inner.rate_limit_throttled_total.inc();
    }

    /// Increment Torznab invalid-request counter with a stable reason label.
    pub fn inc_torznab_invalid_request(&self, reason: &str) {
        self.inner
            .torznab_invalid_requests_total
            .with_label_values(&[reason])
            .inc();
    }

    /// Increment indexer search throughput counter.
    pub fn inc_indexer_search_request(&self, operation: &str, outcome: &str) {
        self.inner
            .indexer_search_requests_total
            .with_label_values(&[operation, outcome])
            .inc();
    }

    /// Increment indexer job outcome counter.
    pub fn inc_indexer_job_outcome(&self, operation: &str, outcome: &str) {
        self.inner
            .indexer_job_outcomes_total
            .with_label_values(&[operation, outcome])
            .inc();
    }

    /// Increment indexer service operation throughput counter.
    pub fn inc_indexer_operation(&self, operation: &str, outcome: &str) {
        self.inner
            .indexer_operations_total
            .with_label_values(&[operation, outcome])
            .inc();
    }

    /// Observe indexer service operation latency in milliseconds.
    pub fn observe_indexer_operation_latency(
        &self,
        operation: &str,
        outcome: &str,
        duration: Duration,
    ) {
        self.inner
            .indexer_operation_latency_ms
            .with_label_values(&[operation, outcome])
            .observe(duration.as_secs_f64() * 1000.0);
    }

    /// Increment discovered media candidate throughput by source and outcome.
    pub fn inc_media_discovery_candidate(&self, source: &str, outcome: &str) {
        self.inner
            .media_discovery_candidates_total
            .with_label_values(&[source, outcome])
            .inc();
    }

    /// Increment queued media jobs by source and dry-run mode.
    pub fn inc_media_job_queued(&self, source: &str, dry_run: bool) {
        self.inner
            .media_jobs_queued_total
            .with_label_values(&[source, dry_run_label(dry_run)])
            .inc();
    }

    /// Increment media capability refresh attempts by outcome.
    pub fn inc_media_capability_refresh(&self, outcome: &str) {
        self.inner
            .media_capability_refresh_total
            .with_label_values(&[outcome])
            .inc();
    }

    /// Increment persisted media job phases by phase and status.
    pub fn inc_media_job_phase(&self, phase: &str, status: &str) {
        self.inner
            .media_job_phases_total
            .with_label_values(&[phase, status])
            .inc();
    }

    /// Increment persisted media job operations by operation kind and outcome.
    pub fn inc_media_job_operation(&self, operation: &str, outcome: &str) {
        self.inner
            .media_job_operations_total
            .with_label_values(&[operation, outcome])
            .inc();
    }

    /// Increment persisted media verification checks by check kind and status.
    pub fn inc_media_job_verification_check(&self, check: &str, status: &str) {
        self.inner
            .media_job_verification_checks_total
            .with_label_values(&[check, status])
            .inc();
    }

    /// Increment media job failures by stable failure category.
    pub fn inc_media_job_failure(&self, category: &str) {
        self.inner
            .media_job_failures_total
            .with_label_values(&[category])
            .inc();
    }

    /// Increment terminal media job outcomes by outcome and dry-run mode.
    pub fn inc_media_job_outcome(&self, outcome: &str, dry_run: bool) {
        self.inner
            .media_job_outcomes_total
            .with_label_values(&[outcome, dry_run_label(dry_run)])
            .inc();
    }

    /// Observe terminal media job duration in milliseconds.
    pub fn observe_media_job_duration(&self, outcome: &str, dry_run: bool, duration: Duration) {
        self.inner
            .media_job_duration_ms
            .with_label_values(&[outcome, dry_run_label(dry_run)])
            .observe(duration.as_secs_f64() * 1000.0);
    }

    /// Increment terminal workspace cleanup outcomes.
    pub fn inc_media_workspace_cleanup(&self, outcome: &str) {
        self.inner
            .media_workspace_cleanup_total
            .with_label_values(&[outcome])
            .inc();
    }

    /// Increment media retention janitor runs by outcome.
    pub fn inc_media_retention_run(&self, outcome: &str) {
        self.inner
            .media_retention_runs_total
            .with_label_values(&[outcome])
            .inc();
    }

    /// Add policy-pruned media retention rows by stable row category.
    pub fn add_media_retention_rows(&self, category: &str, count: u64) {
        self.inner
            .media_retention_rows_total
            .with_label_values(&[category])
            .inc_by(count);
    }

    /// Render the metrics registry using the Prometheus text exposition format.
    ///
    /// # Errors
    ///
    /// Returns an error if the metrics cannot be encoded or if the encoded
    /// buffer is not valid UTF-8.
    pub fn render(&self) -> Result<String> {
        let encoder = TextEncoder::new();
        let metric_families = self.inner.registry.gather();
        let mut buffer = Vec::new();
        encoder
            .encode(&metric_families, &mut buffer)
            .map_err(|source| TelemetryError::MetricsEncode { source })?;
        String::from_utf8(buffer).map_err(|source| TelemetryError::MetricsUtf8 { source })
    }

    /// Take a point-in-time snapshot of the most relevant gauges and counters.
    #[must_use]
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            active_torrents: self.inner.active_torrents.get(),
            queue_depth: self.inner.queue_depth.get(),
            config_watch_latency_ms: self.inner.config_watch_latency_ms.get(),
            config_apply_latency_ms: self.inner.config_apply_latency_ms.get(),
            config_update_failures_total: self.inner.config_update_failures_total.get(),
            config_watch_slow_total: self.inner.config_watch_slow_total.get(),
            guardrail_violations_total: self.inner.guardrail_violations_total.get(),
            rate_limit_throttled_total: self.inner.rate_limit_throttled_total.get(),
        }
    }

    /// Convert a duration to milliseconds saturating at `i64::MAX`.
    pub(crate) fn duration_to_ms(duration: Duration) -> i64 {
        i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
    }
}

impl MetricsInner {
    fn new(registry: Registry) -> Result<Self> {
        let collectors = MetricsCollectors::new()?;
        collectors.register_all(&registry)?;
        let MetricsCollectors {
            http_requests_total,
            events_emitted_total,
            fsops_steps_total,
            active_torrents,
            queue_depth,
            engine_bytes_in,
            engine_bytes_out,
            config_watch_latency_ms,
            config_apply_latency_ms,
            config_update_failures_total,
            config_watch_slow_total,
            guardrail_violations_total,
            rate_limit_throttled_total,
            torznab_invalid_requests_total,
            indexer_search_requests_total,
            indexer_job_outcomes_total,
            indexer_operations_total,
            indexer_operation_latency_ms,
            media_discovery_candidates_total,
            media_jobs_queued_total,
            media_capability_refresh_total,
            media_job_phases_total,
            media_job_operations_total,
            media_job_verification_checks_total,
            media_job_failures_total,
            media_job_outcomes_total,
            media_job_duration_ms,
            media_workspace_cleanup_total,
            media_retention_runs_total,
            media_retention_rows_total,
        } = collectors;

        Ok(Self {
            registry,
            http_requests_total,
            events_emitted_total,
            fsops_steps_total,
            active_torrents,
            queue_depth,
            engine_bytes_in,
            engine_bytes_out,
            config_watch_latency_ms,
            config_apply_latency_ms,
            config_update_failures_total,
            config_watch_slow_total,
            guardrail_violations_total,
            rate_limit_throttled_total,
            torznab_invalid_requests_total,
            indexer_search_requests_total,
            indexer_job_outcomes_total,
            indexer_operations_total,
            indexer_operation_latency_ms,
            media_discovery_candidates_total,
            media_jobs_queued_total,
            media_capability_refresh_total,
            media_job_phases_total,
            media_job_operations_total,
            media_job_verification_checks_total,
            media_job_failures_total,
            media_job_outcomes_total,
            media_job_duration_ms,
            media_workspace_cleanup_total,
            media_retention_runs_total,
            media_retention_rows_total,
        })
    }
}

impl MetricsCollectors {
    fn new() -> Result<Self> {
        let MediaMetricsCollectors {
            discovery_candidates_total,
            jobs_queued_total,
            capability_refresh_total,
            job_phases_total,
            job_operations_total,
            job_verification_checks_total,
            job_failures_total,
            job_outcomes_total,
            job_duration_ms,
            workspace_cleanup_total,
            retention_runs_total,
            retention_rows_total,
        } = media_metrics_collectors()?;

        Ok(Self {
            http_requests_total: counter_vec(
                "http_requests_total",
                "Total HTTP requests received",
                &["route", "code"],
            )?,
            events_emitted_total: counter_vec(
                "events_emitted_total",
                "Domain events emitted by type",
                &["type"],
            )?,
            fsops_steps_total: counter_vec(
                "fsops_steps_total",
                "Filesystem post-processing steps executed by status",
                &["step", "status"],
            )?,
            active_torrents: gauge("active_torrents", "Number of active torrents")?,
            queue_depth: gauge("queue_depth", "Queued torrent operations")?,
            engine_bytes_in: gauge("engine_bytes_in", "Bytes received by the engine")?,
            engine_bytes_out: gauge("engine_bytes_out", "Bytes sent by the engine")?,
            config_watch_latency_ms: gauge(
                "config_watch_latency_ms",
                "Time spent waiting for configuration updates (ms)",
            )?,
            config_apply_latency_ms: gauge(
                "config_apply_latency_ms",
                "Time taken to apply configuration updates (ms)",
            )?,
            config_update_failures_total: counter(
                "config_update_failures_total",
                "Configuration update failures",
            )?,
            config_watch_slow_total: counter(
                "config_watch_slow_total",
                "Configuration updates exceeding the latency guard rail",
            )?,
            guardrail_violations_total: counter(
                "config_guardrail_violations_total",
                "Configuration and setup guardrail violations",
            )?,
            rate_limit_throttled_total: counter(
                "api_rate_limit_throttled_total",
                "Requests rejected due to API rate limiting",
            )?,
            torznab_invalid_requests_total: counter_vec(
                "indexer_torznab_invalid_requests_total",
                "Invalid Torznab requests by reason",
                &["reason"],
            )?,
            indexer_search_requests_total: counter_vec(
                "indexer_search_requests_total",
                "Indexer search request throughput by operation and outcome",
                &["operation", "outcome"],
            )?,
            indexer_job_outcomes_total: counter_vec(
                "indexer_job_outcomes_total",
                "Indexer job outcomes by operation and outcome",
                &["operation", "outcome"],
            )?,
            indexer_operations_total: counter_vec(
                "indexer_operations_total",
                "Indexer service operations by operation and outcome",
                &["operation", "outcome"],
            )?,
            indexer_operation_latency_ms: histogram_vec(
                "indexer_operation_latency_ms",
                "Indexer service operation latency in milliseconds by operation and outcome",
                &["operation", "outcome"],
            )?,
            media_discovery_candidates_total: discovery_candidates_total,
            media_jobs_queued_total: jobs_queued_total,
            media_capability_refresh_total: capability_refresh_total,
            media_job_phases_total: job_phases_total,
            media_job_operations_total: job_operations_total,
            media_job_verification_checks_total: job_verification_checks_total,
            media_job_failures_total: job_failures_total,
            media_job_outcomes_total: job_outcomes_total,
            media_job_duration_ms: job_duration_ms,
            media_workspace_cleanup_total: workspace_cleanup_total,
            media_retention_runs_total: retention_runs_total,
            media_retention_rows_total: retention_rows_total,
        })
    }

    fn register_all(&self, registry: &Registry) -> Result<()> {
        register_collector(
            registry,
            "http_requests_total",
            self.http_requests_total.clone(),
        )?;
        register_collector(
            registry,
            "events_emitted_total",
            self.events_emitted_total.clone(),
        )?;
        register_collector(
            registry,
            "fsops_steps_total",
            self.fsops_steps_total.clone(),
        )?;
        register_collector(registry, "active_torrents", self.active_torrents.clone())?;
        register_collector(registry, "queue_depth", self.queue_depth.clone())?;
        register_collector(registry, "engine_bytes_in", self.engine_bytes_in.clone())?;
        register_collector(registry, "engine_bytes_out", self.engine_bytes_out.clone())?;
        register_collector(
            registry,
            "config_watch_latency_ms",
            self.config_watch_latency_ms.clone(),
        )?;
        register_collector(
            registry,
            "config_apply_latency_ms",
            self.config_apply_latency_ms.clone(),
        )?;
        register_collector(
            registry,
            "config_update_failures_total",
            self.config_update_failures_total.clone(),
        )?;
        register_collector(
            registry,
            "config_watch_slow_total",
            self.config_watch_slow_total.clone(),
        )?;
        register_collector(
            registry,
            "config_guardrail_violations_total",
            self.guardrail_violations_total.clone(),
        )?;
        register_collector(
            registry,
            "api_rate_limit_throttled_total",
            self.rate_limit_throttled_total.clone(),
        )?;
        register_collector(
            registry,
            "indexer_torznab_invalid_requests_total",
            self.torznab_invalid_requests_total.clone(),
        )?;
        register_collector(
            registry,
            "indexer_search_requests_total",
            self.indexer_search_requests_total.clone(),
        )?;
        register_collector(
            registry,
            "indexer_job_outcomes_total",
            self.indexer_job_outcomes_total.clone(),
        )?;
        register_collector(
            registry,
            "indexer_operations_total",
            self.indexer_operations_total.clone(),
        )?;
        register_collector(
            registry,
            "indexer_operation_latency_ms",
            self.indexer_operation_latency_ms.clone(),
        )?;
        self.register_media_collectors(registry)?;
        Ok(())
    }

    fn register_media_collectors(&self, registry: &Registry) -> Result<()> {
        register_collector(
            registry,
            "media_discovery_candidates_total",
            self.media_discovery_candidates_total.clone(),
        )?;
        register_collector(
            registry,
            "media_jobs_queued_total",
            self.media_jobs_queued_total.clone(),
        )?;
        register_collector(
            registry,
            "media_capability_refresh_total",
            self.media_capability_refresh_total.clone(),
        )?;
        register_collector(
            registry,
            "media_job_phases_total",
            self.media_job_phases_total.clone(),
        )?;
        register_collector(
            registry,
            "media_job_operations_total",
            self.media_job_operations_total.clone(),
        )?;
        register_collector(
            registry,
            "media_job_verification_checks_total",
            self.media_job_verification_checks_total.clone(),
        )?;
        register_collector(
            registry,
            "media_job_failures_total",
            self.media_job_failures_total.clone(),
        )?;
        register_collector(
            registry,
            "media_job_outcomes_total",
            self.media_job_outcomes_total.clone(),
        )?;
        register_collector(
            registry,
            "media_job_duration_ms",
            self.media_job_duration_ms.clone(),
        )?;
        register_collector(
            registry,
            "media_workspace_cleanup_total",
            self.media_workspace_cleanup_total.clone(),
        )?;
        register_collector(
            registry,
            "media_retention_runs_total",
            self.media_retention_runs_total.clone(),
        )?;
        register_collector(
            registry,
            "media_retention_rows_total",
            self.media_retention_rows_total.clone(),
        )?;
        Ok(())
    }
}

fn media_metrics_collectors() -> Result<MediaMetricsCollectors> {
    Ok(MediaMetricsCollectors {
        discovery_candidates_total: counter_vec(
            "media_discovery_candidates_total",
            "Media discovery candidates by source and outcome",
            &["source", "outcome"],
        )?,
        jobs_queued_total: counter_vec(
            "media_jobs_queued_total",
            "Queued media jobs by source and dry-run mode",
            &["source", "dry_run"],
        )?,
        capability_refresh_total: counter_vec(
            "media_capability_refresh_total",
            "Media capability refresh attempts by outcome",
            &["outcome"],
        )?,
        job_phases_total: counter_vec(
            "media_job_phases_total",
            "Persisted media job phases by phase and status",
            &["phase", "status"],
        )?,
        job_operations_total: counter_vec(
            "media_job_operations_total",
            "Persisted media job operations by operation kind and outcome",
            &["operation", "outcome"],
        )?,
        job_verification_checks_total: counter_vec(
            "media_job_verification_checks_total",
            "Persisted media job verification checks by check kind and status",
            &["check", "status"],
        )?,
        job_failures_total: counter_vec(
            "media_job_failures_total",
            "Media job failures by stable failure category",
            &["category"],
        )?,
        job_outcomes_total: counter_vec(
            "media_job_outcomes_total",
            "Terminal media job outcomes by outcome and dry-run mode",
            &["outcome", "dry_run"],
        )?,
        job_duration_ms: histogram_vec(
            "media_job_duration_ms",
            "Terminal media job duration in milliseconds by outcome and dry-run mode",
            &["outcome", "dry_run"],
        )?,
        workspace_cleanup_total: counter_vec(
            "media_workspace_cleanup_total",
            "Media job terminal workspace cleanup outcomes",
            &["outcome"],
        )?,
        retention_runs_total: counter_vec(
            "media_retention_runs_total",
            "Media retention janitor runs by outcome",
            &["outcome"],
        )?,
        retention_rows_total: counter_vec(
            "media_retention_rows_total",
            "Rows affected by media retention category",
            &["category"],
        )?,
    })
}

const fn dry_run_label(dry_run: bool) -> &'static str {
    if dry_run { "true" } else { "false" }
}

fn counter_vec(name: &'static str, help: &'static str, labels: &[&str]) -> Result<IntCounterVec> {
    IntCounterVec::new(Opts::new(name, help), labels)
        .map_err(|source| TelemetryError::MetricsCollector { name, source })
}

fn counter(name: &'static str, help: &'static str) -> Result<IntCounter> {
    IntCounter::with_opts(Opts::new(name, help))
        .map_err(|source| TelemetryError::MetricsCollector { name, source })
}

fn histogram_vec(name: &'static str, help: &'static str, labels: &[&str]) -> Result<HistogramVec> {
    HistogramVec::new(HistogramOpts::new(name, help), labels)
        .map_err(|source| TelemetryError::MetricsCollector { name, source })
}

fn gauge(name: &'static str, help: &'static str) -> Result<IntGauge> {
    IntGauge::with_opts(Opts::new(name, help))
        .map_err(|source| TelemetryError::MetricsCollector { name, source })
}

fn register_collector<C>(registry: &Registry, name: &'static str, collector: C) -> Result<()>
where
    C: Collector + Clone + 'static,
{
    registry
        .register(Box::new(collector))
        .map_err(|source| TelemetryError::MetricsRegister { name, source })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn duration_to_ms_saturates_on_large_values() {
        let duration = Duration::from_secs(u64::MAX / 2);
        assert_eq!(Metrics::duration_to_ms(duration), i64::MAX);
    }

    #[test]
    fn metrics_snapshot_reflects_updates() -> Result<()> {
        let metrics = Metrics::new()?;
        metrics.inc_http_request("/health", 200);
        metrics.inc_event("torrent_added");
        metrics.inc_fsops_step("transfer", "completed");
        metrics.set_active_torrents(5);
        metrics.set_queue_depth(2);
        metrics.set_engine_bytes_in(1_024);
        metrics.set_engine_bytes_out(2_048);
        metrics.observe_config_watch_latency(Duration::from_millis(120));
        metrics.observe_config_apply_latency(Duration::from_millis(45));
        metrics.inc_config_update_failure();
        metrics.inc_config_watch_slow();
        metrics.inc_guardrail_violation();
        metrics.inc_rate_limit_throttled();
        metrics.inc_torznab_invalid_request("missing_apikey");
        metrics.inc_indexer_search_request("create", "success");
        metrics.inc_indexer_job_outcome("import_create", "success");
        metrics.inc_indexer_operation("instance_create", "success");
        metrics.observe_indexer_operation_latency(
            "instance_create",
            "success",
            Duration::from_millis(75),
        );
        metrics.inc_media_discovery_candidate("manual", "queued");
        metrics.inc_media_job_queued("manual", true);
        metrics.inc_media_capability_refresh("success");
        metrics.inc_media_job_phase("inspect_plan", "completed");
        metrics.inc_media_job_operation("video_transcode", "planned");
        metrics.inc_media_job_verification_check("candidate_graph", "passed");
        metrics.inc_media_job_failure("verification");
        metrics.inc_media_job_outcome("completed", true);
        metrics.observe_media_job_duration("completed", true, Duration::from_millis(250));
        metrics.inc_media_workspace_cleanup("success");
        metrics.inc_media_retention_run("success");
        metrics.add_media_retention_rows("completed_jobs", 2);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.active_torrents, 5);
        assert_eq!(snapshot.queue_depth, 2);
        assert_eq!(snapshot.config_watch_latency_ms, 120);
        assert_eq!(snapshot.config_apply_latency_ms, 45);
        assert_eq!(snapshot.config_update_failures_total, 1);
        assert_eq!(snapshot.config_watch_slow_total, 1);
        assert_eq!(snapshot.guardrail_violations_total, 1);
        assert_eq!(snapshot.rate_limit_throttled_total, 1);

        let rendered = metrics.render()?;
        assert!(rendered.contains("http_requests_total"));
        assert!(rendered.contains("fsops_steps_total"));
        assert!(rendered.contains("config_guardrail_violations_total"));
        assert!(rendered.contains("indexer_torznab_invalid_requests_total"));
        assert!(rendered.contains("indexer_search_requests_total"));
        assert!(rendered.contains("indexer_job_outcomes_total"));
        assert!(rendered.contains("indexer_operations_total"));
        assert!(rendered.contains("indexer_operation_latency_ms"));
        assert!(rendered.contains("media_discovery_candidates_total"));
        assert!(rendered.contains("media_jobs_queued_total"));
        assert!(rendered.contains("media_capability_refresh_total"));
        assert!(rendered.contains("media_job_phases_total"));
        assert!(rendered.contains("media_job_operations_total"));
        assert!(rendered.contains("media_job_verification_checks_total"));
        assert!(rendered.contains("media_job_failures_total"));
        assert!(rendered.contains("media_job_outcomes_total"));
        assert!(rendered.contains("media_job_duration_ms"));
        assert!(rendered.contains("media_workspace_cleanup_total"));
        assert!(rendered.contains("media_retention_runs_total"));
        assert!(rendered.contains("media_retention_rows_total"));
        Ok(())
    }

    #[test]
    fn metric_constructor_helpers_report_invalid_descriptors() {
        assert!(matches!(
            counter("", "invalid"),
            Err(TelemetryError::MetricsCollector { name: "", .. })
        ));
        assert!(matches!(
            counter_vec("invalid_counter_vec", "invalid", &[""]),
            Err(TelemetryError::MetricsCollector {
                name: "invalid_counter_vec",
                ..
            })
        ));
        assert!(matches!(
            gauge("", "invalid"),
            Err(TelemetryError::MetricsCollector { name: "", .. })
        ));
        assert!(matches!(
            histogram_vec("invalid_histogram_vec", "invalid", &[""]),
            Err(TelemetryError::MetricsCollector {
                name: "invalid_histogram_vec",
                ..
            })
        ));
    }

    #[test]
    fn collector_registration_reports_duplicate_names() -> Result<()> {
        let registry = Registry::new();
        let collector = counter("duplicate_counter_total", "Duplicate counter")?;
        register_collector(&registry, "duplicate_counter_total", collector.clone())?;

        assert!(matches!(
            register_collector(&registry, "duplicate_counter_total", collector),
            Err(TelemetryError::MetricsRegister {
                name: "duplicate_counter_total",
                ..
            })
        ));
        Ok(())
    }
}
