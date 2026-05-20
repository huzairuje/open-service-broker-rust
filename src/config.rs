//! Configuration loaded from environment variables.

use std::env;

/// Which storage backend to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageBackend {
    Memory,
    Postgres,
}

/// Runtime configuration for the broker.
#[derive(Debug, Clone)]
pub struct Config {
    /// Bind address.
    pub host: String,
    /// Bind port.
    pub port: u16,
    /// HTTP Basic Auth username expected on every request.
    pub username: String,
    /// HTTP Basic Auth password expected on every request.
    pub password: String,
    /// Minimum OSB API version this broker accepts (e.g., "2.13").
    pub min_api_version: String,
    /// Optional path to a catalog JSON/YAML file. If unset, the
    /// built-in sample catalog is used.
    pub catalog_path: Option<String>,
    /// Storage backend to use.
    pub storage: StorageBackend,
    /// Postgres connection URL (only required when `storage = Postgres`).
    pub database_url: Option<String>,
    /// Simulated async operation duration, in milliseconds. Set to 0
    /// to provision synchronously.
    pub async_op_millis: u64,
}

impl Config {
    /// Build a `Config` by reading the standard `BROKER_*` env vars.
    pub fn from_env() -> Self {
        let storage = match env::var("BROKER_STORAGE")
            .unwrap_or_else(|_| "memory".into())
            .to_ascii_lowercase()
            .as_str()
        {
            "postgres" | "pg" => StorageBackend::Postgres,
            _ => StorageBackend::Memory,
        };

        Self {
            host: env::var("BROKER_HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            port: env::var("BROKER_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(8080),
            username: env::var("BROKER_USERNAME").unwrap_or_else(|_| "admin".into()),
            password: env::var("BROKER_PASSWORD").unwrap_or_else(|_| "password".into()),
            min_api_version: env::var("BROKER_MIN_API_VERSION").unwrap_or_else(|_| "2.13".into()),
            catalog_path: env::var("BROKER_CATALOG_PATH").ok(),
            storage,
            database_url: env::var("DATABASE_URL").ok(),
            async_op_millis: env::var("BROKER_ASYNC_OP_MILLIS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::from_env()
    }
}
