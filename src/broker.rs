//! Core broker logic. Owns the catalog, the operation tracker, and
//! delegates persistence to a `Storage` implementation.

use std::sync::Arc;

use crate::catalog_loader;
use crate::error::{BrokerError, BrokerResult};
use crate::models::catalog::{Catalog, Plan, Service};
use crate::operations::OperationTracker;
use crate::storage::Storage;

/// The broker ties together the static catalog, the operation tracker
/// for async work, and the dynamic state held in `Storage`.
pub struct Broker {
    catalog: Catalog,
    storage: Arc<dyn Storage>,
    operations: Arc<OperationTracker>,
}

impl Broker {
    /// Construct a broker using the default sample catalog.
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self::with_catalog(storage, catalog_loader::default_catalog())
    }

    /// Construct a broker with a caller-supplied catalog.
    pub fn with_catalog(storage: Arc<dyn Storage>, catalog: Catalog) -> Self {
        Self {
            catalog,
            storage,
            operations: Arc::new(OperationTracker::new()),
        }
    }

    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    pub fn storage(&self) -> &Arc<dyn Storage> {
        &self.storage
    }

    pub fn operations(&self) -> &Arc<OperationTracker> {
        &self.operations
    }

    /// Look up a service by id. Used to validate provision/bind requests.
    pub fn find_service(&self, service_id: &str) -> BrokerResult<&Service> {
        self.catalog
            .services
            .iter()
            .find(|s| s.id == service_id)
            .ok_or_else(|| BrokerError::BadRequest(format!("unknown service_id: {service_id}")))
    }

    /// Look up a plan within a service.
    pub fn find_plan<'a>(&'a self, service: &'a Service, plan_id: &str) -> BrokerResult<&'a Plan> {
        service
            .plans
            .iter()
            .find(|p| p.id == plan_id)
            .ok_or_else(|| BrokerError::BadRequest(format!("unknown plan_id: {plan_id}")))
    }
}
