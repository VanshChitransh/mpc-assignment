use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration, timeout};
use anyhow::Result;
use tracing::{info, error, warn, debug};
use config::{Config, ConfigError};
use serde::Deserialize;

mod database;
mod processor;
mod yellowstone;
mod geyser;

use database::DatabaseManager;
use processor::TransactionProcessor;
use yellowstone::YellowstoneClient;

#[derive(Debug, Deserialize, Clone)]
struct AppConfig {
    #[serde(rename = "DATABASE_URL")]
    database_url: String,

    #[serde(rename = "YELLOWSTONE_ENDPOINT")]
    yellowstone_endpoint: String,

    #[serde(rename = "YELLOWSTONE_TOKEN")]
    yellowstone_token: Option<String>,

    #[serde(rename = "COMMITMENT_LEVEL")]
    commitment_level: Option<String>,

    #[serde(rename = "RECONNECT_INTERVAL")]
    reconnect_interval: Option<u64>,

    #[serde(rename = "MAX_RECONNECT_ATTEMPTS")]
    max_reconnect_attempts: Option<u32>,

    #[serde(rename = "HEALTH_CHECK_INTERVAL_SECONDS")]
    health_check_interval_seconds: Option<u64>,

    #[serde(rename = "BATCH_SIZE")]
    batch_size: Option<usize>,

    #[serde(rename = "LOG_LEVEL")]
    log_level: Option<String>,
}

impl AppConfig {
    fn from_env() -> Result<Self, ConfigError> {
        // Load environment variables with dotenvy already initialized
        let settings = Config::builder()
            .add_source(config::Environment::default().separator("_"))
            .build()?;

        settings.try_deserialize()
    }
}

#[derive(Debug)]
pub struct IndexerSubscriptionManager {
    client: Arc<RwLock<YellowstoneClient>>,
    active_subscriptions: usize,
    total_updates_processed: u64,
}

impl IndexerSubscriptionManager {
    pub fn new(client: YellowstoneClient) -> Self {
        Self {
            client: Arc::new(RwLock::new(client)),
            active_subscriptions: 0,
            total_updates_processed: 0,
        }
    }

    pub async fn subscribe_to_addresses(&mut self, addresses: Vec<String>) -> Result<()> {
        let mut client = self.client.write().await;
        client.subscribe_to_addresses(addresses).await?;
        self.active_subscriptions = 1;
        Ok(())
    }

    pub async fn next_update(&mut self) -> Result<Option<Vec<yellowstone::YellowstoneUpdate>>> {
        let mut client = self.client.write().await;
        match client.next_update().await? {
            Some(update) => {
                self.total_updates_processed += 1;
                Ok(Some(vec![update]))
            }
            None => Ok(None),
        }
    }

    pub async fn get_stats(&self) -> IndexerSubscriptionStats {
        IndexerSubscriptionStats {
            active_subscriptions: self.active_subscriptions,
            total_updates_processed: self.total_updates_processed,
        }
    }
}

#[derive(Debug)]
pub struct IndexerSubscriptionStats {
    pub active_subscriptions: usize,
    pub total_updates_processed: u64,
}

#[derive(Clone)]
struct AppState {
    database: DatabaseManager,
    subscription_manager: Arc<RwLock<IndexerSubscriptionManager>>,
    processor: Arc<TransactionProcessor>,
    config: AppConfig,
}

async fn initialize_tracing(level: Option<&str>) -> Result<()> {
    let level = level.unwrap_or("info");

    tracing_subscriber::fmt()
        .with_env_filter(format!("solana_wallet_indexer={}", level))
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    info!("Initialized tracing with level: {}", level);
    Ok(())
}

async fn initialize_database(database_url: &str) -> Result<DatabaseManager> {
    info!("Connecting to database...");
    let pool = sqlx::PgPool::connect(database_url).await?;

    info!("Running database migrations...");
    sqlx::migrate!("./migrations").run(&pool).await?;

    let database = DatabaseManager::new(pool);
    info!("Database initialized successfully");
    Ok(database)
}

async fn initialize_yellowstone(config: &AppConfig) -> Result<YellowstoneClient> {
    info!("Initializing Yellowstone GRPC client...");
    let client = YellowstoneClient::new(
        &config.yellowstone_endpoint,
        config.yellowstone_token.clone(),
    )
    .await?;

    info!("Yellowstone client initialized successfully");
    Ok(client)
}

// Health check loop
async fn health_check_loop(state: AppState) {
    let interval = Duration::from_secs(
        state.config.health_check_interval_seconds.unwrap_or(30),
    );

    info!("Starting health check loop (interval: {:?})", interval);

    loop {
        sleep(interval).await;

        match state.database.health_check().await {
            Ok(true) => debug!("Database health check: OK"),
            Ok(false) => warn!("Database health check: NOT OK"),
            Err(e) => error!("Database health check error: {}", e),
        }

        {
            let manager = state.subscription_manager.read().await;
            let stats = manager.get_stats().await;
            info!(
                "Subscription stats: active={}, total_updates={}",
                stats.active_subscriptions, stats.total_updates_processed
            );
        }
    }
}

// Main processing loop
async fn process_updates_loop(state: AppState) -> Result<()> {
    info!("Starting update processing loop...");
    let batch_size = state.config.batch_size.unwrap_or(100);

    loop {
        let addresses = state.database.get_monitoring_addresses().await?;

        if addresses.is_empty() {
            info!("No addresses to monitor, sleeping...");
            sleep(Duration::from_secs(10)).await;
            continue;
        }

        info!("Monitoring {} addresses", addresses.len());

        {
            let mut manager = state.subscription_manager.write().await;
            match manager.subscribe_to_addresses(addresses.clone()).await {
                Ok(_) => info!("Successfully subscribed to addresses"),
                Err(e) => {
                    error!("Failed to subscribe to addresses: {}", e);
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            }
        }

        let mut update_count = 0;
        let timeout_duration = Duration::from_secs(30);

        loop {
            let updates_result = {
                let mut manager = state.subscription_manager.write().await;
                timeout(timeout_duration, manager.next_update()).await
            };

            match updates_result {
                Ok(Ok(Some(updates))) => {
                    for update in updates {
                        if let Err(e) = state.processor.process_update(update).await {
                            error!("Failed to process update: {}", e);
                        } else {
                            update_count += 1;
                        }

                        if update_count >= batch_size {
                            debug!("Processed {} updates", update_count);
                            update_count = 0;
                            sleep(Duration::from_millis(100)).await;
                        }
                    }
                }
                Ok(Ok(None)) => {
                    debug!("Received empty update, continuing...");
                    continue;
                }
                Ok(Err(e)) => {
                    error!("Error receiving updates: {}", e);
                    break;
                }
                Err(_) => {
                    debug!("Update timeout, checking for new addresses...");
                    break;
                }
            }
        }

        sleep(Duration::from_secs(1)).await;
    }
}

// Metrics reporting loop
async fn metrics_loop(state: AppState) {
    let interval = Duration::from_secs(60);
    info!("Starting metrics reporting loop");

    loop {
        sleep(interval).await;

        match state.database.get_stats().await {
            Ok(stats) => {
                info!(
                    "Database stats - Wallets: {}, Changes: {}, Total Balance: {} SOL",
                    stats.wallet_count, stats.balance_changes_24h, stats.total_balance
                );

                let _ = state
                    .database
                    .record_metric("total_wallets", stats.wallet_count, None)
                    .await;

                let _ = state
                    .database
                    .record_metric("balance_changes_24h", stats.balance_changes_24h, None)
                    .await;
            }
            Err(e) => error!("Failed to collect database stats: {}", e),
        }

        {
            let manager = state.subscription_manager.read().await;
            let sub_stats = manager.get_stats().await;
            info!(
                "Subscription stats - Active: {}, Updates: {}",
                sub_stats.active_subscriptions, sub_stats.total_updates_processed
            );
        }
    }
}

// Cleanup loop
async fn cleanup_loop(state: AppState) {
    let interval = Duration::from_secs(3600);
    info!("Starting cleanup loop");

    loop {
        sleep(interval).await;

        info!("Running cleanup tasks...");

        match state.database.cleanup_old_data(30).await {
            Ok(deleted_count) => {
                if deleted_count > 0 {
                    info!("Cleaned up {} old balance change records", deleted_count);
                }
            }
            Err(e) => error!("Cleanup failed: {}", e),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env variables before reading AppConfig
    dotenvy::dotenv().ok();

    let config = AppConfig::from_env()
        .map_err(|e| anyhow::anyhow!("Failed to load configuration: {}", e))?;

    initialize_tracing(config.log_level.as_deref()).await?;

    info!("Starting Solana Wallet Indexer - Phase 4");
    info!("Configuration loaded successfully");

    let database = initialize_database(&config.database_url).await?;
    let yellowstone_client = initialize_yellowstone(&config).await?;
    let processor = Arc::new(TransactionProcessor::new(database.pool.clone()));
    let subscription_manager = Arc::new(RwLock::new(
        IndexerSubscriptionManager::new(yellowstone_client),
    ));

    let state = AppState {
        database: database.clone(),
        subscription_manager: subscription_manager.clone(),
        processor: processor.clone(),
        config: config.clone(),
    };

    info!("All components initialized successfully");

    // Spawn loops
    let health_check_state = state.clone();
    let health_check_handle = tokio::spawn(async move {
        health_check_loop(health_check_state).await;
    });

    let metrics_state = state.clone();
    let metrics_handle = tokio::spawn(async move {
        metrics_loop(metrics_state).await;
    });

    let cleanup_state = state.clone();
    let cleanup_handle = tokio::spawn(async move {
        cleanup_loop(cleanup_state).await;
    });

    info!("Starting main processing loop...");
    let processing_result = process_updates_loop(state).await;

    error!("Main processing loop exited: {:?}", processing_result);

    health_check_handle.abort();
    metrics_handle.abort();
    cleanup_handle.abort();

    sleep(Duration::from_secs(2)).await;

    info!("Solana Wallet Indexer shutdown complete");
    Ok(())
}
