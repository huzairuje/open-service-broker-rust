//! Storage abstraction for service instances and bindings.
//!
//! Decoupling storage from the broker logic lets us swap in-memory,
//! SQL, or Redis backends without touching handlers.

pub mod memory;
#[cfg(feature = "postgres")]
pub mod postgres;

use async_trait::async_trait;

use crate::error::BrokerResult;
use crate::models::service_binding::ServiceBinding;
use crate::models::service_instance::ServiceInstance;

/// Persistence operations the broker needs.
#[async_trait]
pub trait Storage: Send + Sync {
    // --- service instances ---

    async fn put_instance(&self, instance: ServiceInstance) -> BrokerResult<()>;
    async fn get_instance(&self, id: &str) -> BrokerResult<Option<ServiceInstance>>;
    async fn delete_instance(&self, id: &str) -> BrokerResult<bool>;

    // --- service bindings ---

    async fn put_binding(&self, binding: ServiceBinding) -> BrokerResult<()>;
    async fn get_binding(&self, id: &str) -> BrokerResult<Option<ServiceBinding>>;
    async fn delete_binding(&self, id: &str) -> BrokerResult<bool>;
}
