//! Binary entry point. Loads config, builds the broker, and runs the
//! axum HTTP server.

use std::sync::Arc;

use rust_open_service_broker::{
    broker::Broker,
    catalog_loader,
    config::{Config, StorageBackend},
    storage::{memory::MemoryStorage, Storage},
    AppState,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Logging: respect RUST_LOG env var, default to info.
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Arc::new(Config::from_env());

    // Pick a catalog source: file (if configured) or built-in sample.
    let catalog = match &config.catalog_path {
        Some(path) => {
            tracing::info!("loading catalog from {}", path);
            catalog_loader::load_from_file(path)?
        }
        None => {
            tracing::info!("using built-in sample catalog");
            catalog_loader::default_catalog()
        }
    };

    let storage: Arc<dyn Storage> = build_storage(&config).await?;
    let broker = Arc::new(Broker::with_catalog(storage, catalog));

    let state = AppState {
        broker,
        config: config.clone(),
    };

    let app = rust_open_service_broker::build_router(state);

    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("rust-open-service-broker listening on {}", addr);

    axum::serve(listener, app).await?;
    Ok(())
}

async fn build_storage(config: &Config) -> Result<Arc<dyn Storage>, Box<dyn std::error::Error>> {
    match config.storage {
        StorageBackend::Memory => {
            tracing::info!("storage backend: in-memory");
            Ok(Arc::new(MemoryStorage::new()))
        }
        StorageBackend::Postgres => {
            #[cfg(feature = "postgres")]
            {
                let url = config
                    .database_url
                    .as_ref()
                    .ok_or("BROKER_STORAGE=postgres requires DATABASE_URL to be set")?;
                tracing::info!("storage backend: postgres");
                let pg = rust_open_service_broker::storage::postgres::PostgresStorage::connect(url)
                    .await?;
                pg.migrate().await?;
                Ok(Arc::new(pg))
            }
            #[cfg(not(feature = "postgres"))]
            {
                let _ = config;
                Err("postgres backend requested but binary built without `postgres` feature".into())
            }
        }
    }
}
