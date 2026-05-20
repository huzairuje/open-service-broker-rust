//! In-memory tracker for asynchronous OSB operations.
//!
//! When the broker accepts an async provision/update/deprovision/bind,
//! it returns an opaque `operation` token. The platform later polls
//! `last_operation` with that token; we look it up here.

use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use uuid::Uuid;

use crate::models::common::OperationState;

/// Kind of resource an operation targets. Used to scope the token so a
/// binding op can't be confused with an instance op.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    Instance,
    Binding,
}

/// What the operation is doing. Recorded for debugging and shown in
/// `last_operation` responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationKind {
    Provision,
    Update,
    Deprovision,
    Bind,
    Unbind,
}

#[derive(Debug, Clone)]
pub struct Operation {
    pub id: String,
    pub resource_kind: ResourceKind,
    pub resource_id: String,
    pub kind: OperationKind,
    pub state: OperationState,
    pub description: String,
}

/// Concurrent registry of outstanding operations.
#[derive(Default)]
pub struct OperationTracker {
    ops: DashMap<String, Operation>,
}

impl OperationTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new in-progress operation and return its token.
    pub fn start(
        &self,
        resource_kind: ResourceKind,
        resource_id: &str,
        kind: OperationKind,
    ) -> String {
        let id = Uuid::new_v4().to_string();
        self.ops.insert(
            id.clone(),
            Operation {
                id: id.clone(),
                resource_kind,
                resource_id: resource_id.to_string(),
                kind,
                state: OperationState::InProgress,
                description: format!("{kind:?} in progress"),
            },
        );
        id
    }

    /// Mark an operation as completed (succeeded or failed).
    pub fn finish(&self, op_id: &str, state: OperationState, description: &str) {
        if let Some(mut entry) = self.ops.get_mut(op_id) {
            entry.state = state;
            entry.description = description.to_string();
        }
    }

    /// Look up an operation by token.
    pub fn get(&self, op_id: &str) -> Option<Operation> {
        self.ops.get(op_id).map(|e| e.clone())
    }

    /// Find the most recent operation targeting a given resource. Used
    /// when the platform polls `last_operation` without supplying the
    /// token (the OSB spec allows that).
    pub fn latest_for(&self, kind: ResourceKind, resource_id: &str) -> Option<Operation> {
        self.ops
            .iter()
            .filter(|e| e.resource_kind == kind && e.resource_id == resource_id)
            .map(|e| e.clone())
            .next()
    }
}

/// Spawn a background task that finishes an operation after `delay`.
/// `on_done` runs first (e.g., to mutate storage), then the operation
/// is marked succeeded.
pub fn spawn_simulated<F>(
    tracker: Arc<OperationTracker>,
    op_id: String,
    delay: Duration,
    on_done: F,
) where
    F: FnOnce() + Send + 'static,
{
    tokio::spawn(async move {
        tokio::time::sleep(delay).await;
        on_done();
        tracker.finish(&op_id, OperationState::Succeeded, "operation completed");
    });
}
