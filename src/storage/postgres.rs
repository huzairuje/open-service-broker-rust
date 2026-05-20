//! Postgres-backed `Storage` implementation.
//!
//! Schema is created on first connect via `migrate()`. Parameters and
//! credentials are stored as `JSONB`.
//!
//! Compiled only when the `postgres` cargo feature is enabled.

use async_trait::async_trait;
use serde_json::Value;
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Row;

use crate::error::{BrokerError, BrokerResult};
use crate::models::service_binding::ServiceBinding;
use crate::models::service_instance::ServiceInstance;
use crate::storage::Storage;

const SCHEMA_STATEMENTS: &[&str] = &[
    r#"
CREATE TABLE IF NOT EXISTS service_instances (
    id              TEXT PRIMARY KEY,
    service_id      TEXT NOT NULL,
    plan_id         TEXT NOT NULL,
    parameters      JSONB,
    context         JSONB,
    dashboard_url   TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
)
"#,
    r#"
CREATE TABLE IF NOT EXISTS service_bindings (
    id              TEXT PRIMARY KEY,
    instance_id     TEXT NOT NULL REFERENCES service_instances(id) ON DELETE CASCADE,
    service_id      TEXT NOT NULL,
    plan_id         TEXT NOT NULL,
    credentials     JSONB NOT NULL,
    parameters      JSONB,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
)
"#,
];

/// Postgres-backed storage. Cheap to clone; wraps an `sqlx::PgPool`.
#[derive(Clone)]
pub struct PostgresStorage {
    pool: PgPool,
}

impl PostgresStorage {
    /// Open a pooled connection to the given database URL.
    pub async fn connect(url: &str) -> BrokerResult<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(url)
            .await
            .map_err(|e| BrokerError::Internal(format!("postgres connect: {e}")))?;
        Ok(Self { pool })
    }

    /// Construct from an existing pool (handy for tests).
    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Apply the schema. Idempotent.
    pub async fn migrate(&self) -> BrokerResult<()> {
        for stmt in SCHEMA_STATEMENTS {
            sqlx::query(stmt)
                .execute(&self.pool)
                .await
                .map_err(|e| BrokerError::Internal(format!("postgres migrate: {e}")))?;
        }
        Ok(())
    }
}

fn db_err(ctx: &str, e: sqlx::Error) -> BrokerError {
    BrokerError::Internal(format!("{ctx}: {e}"))
}

#[async_trait]
impl Storage for PostgresStorage {
    async fn put_instance(&self, instance: ServiceInstance) -> BrokerResult<()> {
        sqlx::query(
            r#"
            INSERT INTO service_instances
                (id, service_id, plan_id, parameters, context, dashboard_url, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, now())
            ON CONFLICT (id) DO UPDATE SET
                service_id = EXCLUDED.service_id,
                plan_id = EXCLUDED.plan_id,
                parameters = EXCLUDED.parameters,
                context = EXCLUDED.context,
                dashboard_url = EXCLUDED.dashboard_url,
                updated_at = now()
            "#,
        )
        .bind(&instance.id)
        .bind(&instance.service_id)
        .bind(&instance.plan_id)
        .bind(instance.parameters.clone())
        .bind(instance.context.clone())
        .bind(instance.dashboard_url.as_deref())
        .execute(&self.pool)
        .await
        .map_err(|e| db_err("put_instance", e))?;
        Ok(())
    }

    async fn get_instance(&self, id: &str) -> BrokerResult<Option<ServiceInstance>> {
        let row = sqlx::query(
            r#"SELECT id, service_id, plan_id, parameters, context, dashboard_url
               FROM service_instances WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| db_err("get_instance", e))?;

        Ok(row.map(|r| ServiceInstance {
            id: r.get("id"),
            service_id: r.get("service_id"),
            plan_id: r.get("plan_id"),
            parameters: r.get::<Option<Value>, _>("parameters"),
            context: r.get::<Option<Value>, _>("context"),
            dashboard_url: r.get("dashboard_url"),
        }))
    }

    async fn delete_instance(&self, id: &str) -> BrokerResult<bool> {
        let result = sqlx::query("DELETE FROM service_instances WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| db_err("delete_instance", e))?;
        Ok(result.rows_affected() > 0)
    }

    async fn put_binding(&self, binding: ServiceBinding) -> BrokerResult<()> {
        let creds = serde_json::to_value(&binding.credentials)
            .map_err(|e| BrokerError::Internal(format!("serialize credentials: {e}")))?;
        sqlx::query(
            r#"
            INSERT INTO service_bindings
                (id, instance_id, service_id, plan_id, credentials, parameters, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, now())
            ON CONFLICT (id) DO UPDATE SET
                instance_id = EXCLUDED.instance_id,
                service_id = EXCLUDED.service_id,
                plan_id = EXCLUDED.plan_id,
                credentials = EXCLUDED.credentials,
                parameters = EXCLUDED.parameters,
                updated_at = now()
            "#,
        )
        .bind(&binding.id)
        .bind(&binding.instance_id)
        .bind(&binding.service_id)
        .bind(&binding.plan_id)
        .bind(creds)
        .bind(binding.parameters.clone())
        .execute(&self.pool)
        .await
        .map_err(|e| db_err("put_binding", e))?;
        Ok(())
    }

    async fn get_binding(&self, id: &str) -> BrokerResult<Option<ServiceBinding>> {
        let row = sqlx::query(
            r#"SELECT id, instance_id, service_id, plan_id, credentials, parameters
               FROM service_bindings WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| db_err("get_binding", e))?;

        let Some(r) = row else {
            return Ok(None);
        };
        let creds_value: Value = r.get("credentials");
        let credentials = serde_json::from_value(creds_value)
            .map_err(|e| BrokerError::Internal(format!("decode credentials: {e}")))?;

        Ok(Some(ServiceBinding {
            id: r.get("id"),
            instance_id: r.get("instance_id"),
            service_id: r.get("service_id"),
            plan_id: r.get("plan_id"),
            credentials,
            parameters: r.get::<Option<Value>, _>("parameters"),
        }))
    }

    async fn delete_binding(&self, id: &str) -> BrokerResult<bool> {
        let result = sqlx::query("DELETE FROM service_bindings WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| db_err("delete_binding", e))?;
        Ok(result.rows_affected() > 0)
    }
}
