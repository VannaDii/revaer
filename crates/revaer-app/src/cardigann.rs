//! The explicitly supported public HTML Cardigann subset, not a template interpreter.

mod definition;
mod html;

pub(crate) use definition::prepare;
pub(crate) use html::{fetch, fetch_recent};

/// Configuration diagnostics are fixed strings; YAML, query and upstream text are never errors.
pub(crate) const REIMPORT_REQUIRED: &str =
    "Reimport this Cardigann definition with supported public GET HTML search rules";

#[cfg(test)]
pub(crate) mod tests;
