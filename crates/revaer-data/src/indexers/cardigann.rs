//! Normalized Cardigann rules, published with definition metadata in one transaction.

use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::manual_search::ManualSearchRun;
use crate::error::{Result, try_op};

/// Executable public GET rules. Arrays are ordered query/bind transport, never stored blobs.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CardigannPlan {
    /// Definition's first public link.
    pub origin: String,
    /// Single static path relative to that link.
    pub search_path: String,
    /// CSS selector for release rows.
    pub rows_selector: String,
    /// Literal HTTP query parameter names.
    pub input_names: Vec<String>,
    /// Paired parameter values, allowing only keyword substitution.
    pub input_values: Vec<String>,
    /// Supported release field names.
    pub field_names: Vec<String>,
    /// Paired CSS field selectors.
    pub field_selectors: Vec<String>,
    /// Paired optional attribute names; absence means element text.
    pub field_attributes: Vec<Option<String>>,
    /// Upstream category keys, including symbolic keys.
    pub category_sources: Vec<String>,
    /// Paired standard category IDs from the existing category catalog.
    pub category_targets: Vec<i32>,
}

/// Publish validated rules inside the definition import's existing transaction.
///
/// # Errors
/// Returns authorization, category-catalog, validation or database errors.
pub async fn import(
    tx: &mut Transaction<'_, Postgres>,
    actor: Uuid,
    slug: &str,
    plan: &CardigannPlan,
) -> Result<()> {
    try_op("Cardigann rules import", || async {
        sqlx::query("SELECT cardigann_search_import_v1(actor_input => $1, slug_input => $2, origin_input => $3, path_input => $4, rows_input => $5, input_names => $6, input_values => $7, field_names => $8, field_selectors => $9, field_attributes => $10, category_sources => $11, category_targets => $12)")
            .bind(actor)
            .bind(slug)
            .bind(&plan.origin)
            .bind(&plan.search_path)
            .bind(&plan.rows_selector)
            .bind(&plan.input_names)
            .bind(&plan.input_values)
            .bind(&plan.field_names)
            .bind(&plan.field_selectors)
            .bind(&plan.field_attributes)
            .bind(&plan.category_sources)
            .bind(&plan.category_targets)
            .execute(&mut **tx)
            .await?;
        Ok(())
    })
    .await
}

/// Read one atomic rule snapshot for an owned, still-active search attempt.
/// `None` means metadata-only import or lost ownership; no network work is authorized.
///
/// # Errors
/// Returns database failures without exposing upstream input.
pub async fn load(pool: &PgPool, run: &ManualSearchRun) -> Result<Option<CardigannPlan>> {
    try_op("Cardigann rules load", || {
        sqlx::query_as("SELECT * FROM cardigann_search_plan_v1(request_input => $1, instance_input => $2, token_input => $3)")
            .bind(run.search_request_public_id)
            .bind(run.indexer_instance_public_id)
            .bind(run.claim_token)
            .fetch_optional(pool)
    })
    .await
}
