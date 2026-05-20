//! Axum handlers, one module per OSB resource.

pub mod catalog;
pub mod service_binding;
pub mod service_instance;

use serde::Deserialize;

/// Common query string supported by most OSB endpoints.
#[derive(Debug, Deserialize, Default)]
pub struct AcceptsIncomplete {
    /// Platform signal that it accepts async (`202`) responses.
    #[serde(default)]
    pub accepts_incomplete: Option<bool>,
}
