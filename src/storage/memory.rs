//! In-memory storage backend.
//!
//! Backed by `dashmap` for lock-free concurrent reads/writes. Suitable
//! for development and tests; replace with a persistent backend for
//! production.

use async_trait::async_trait;
use dashmap::DashMap;

use crate::error::BrokerResult;
use crate::models::service_binding::ServiceBinding;
use crate::models::service_instance::ServiceInstance;
use crate::storage::Storage;

#[derive(Default)]
pub struct MemoryStorage {
    instances: DashMap<String, ServiceInstance>,
    bindings: DashMap<String, ServiceBinding>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl Storage for MemoryStorage {
    async fn put_instance(&self, instance: ServiceInstance) -> BrokerResult<()> {
        self.instances.insert(instance.id.clone(), instance);
        Ok(())
    }

    async fn get_instance(&self, id: &str) -> BrokerResult<Option<ServiceInstance>> {
        Ok(self.instances.get(id).map(|v| v.clone()))
    }

    async fn delete_instance(&self, id: &str) -> BrokerResult<bool> {
        Ok(self.instances.remove(id).is_some())
    }

    async fn put_binding(&self, binding: ServiceBinding) -> BrokerResult<()> {
        self.bindings.insert(binding.id.clone(), binding);
        Ok(())
    }

    async fn get_binding(&self, id: &str) -> BrokerResult<Option<ServiceBinding>> {
        Ok(self.bindings.get(id).map(|v| v.clone()))
    }

    async fn delete_binding(&self, id: &str) -> BrokerResult<bool> {
        Ok(self.bindings.remove(id).is_some())
    }
}
