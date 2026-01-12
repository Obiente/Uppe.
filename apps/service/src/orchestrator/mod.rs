/// Orchestrator module - coordinates all components
///
/// The orchestrator is the core coordinator that:
/// - Manages the lifecycle of all components
/// - Coordinates between monitoring, database, crypto, and P2P layers
/// - Handles results and distributes them appropriately
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::config::Config;
use crate::crypto::{KeyPair, load_or_generate_keypair, sign_result};
use crate::database::{Database, DatabaseImpl, initialize_database};
use crate::monitoring::checker::CheckType;
use crate::monitoring::scheduler::MonitorConfig;
use crate::monitoring::{CheckResult, MonitoringExecutor, MonitoringScheduler};
use crate::p2p::P2PNetwork;
use crate::pool::LibsqlPool;

/// Main orchestrator for the Uppe service
pub struct Orchestrator {
    #[allow(dead_code)] // Will be used for runtime configuration changes
    config: Arc<Config>,
    database: Arc<dyn Database>,
    keypair: Arc<KeyPair>,
    executor: Arc<MonitoringExecutor>,
    p2p_network: Arc<P2PNetwork>,
    task_handles: Vec<tokio::task::JoinHandle<()>>,
}

impl Orchestrator {
    /// Create and start a new orchestrator
    /// This is a convenience method that creates and immediately runs the orchestrator
    pub async fn start(config: Config, pool: LibsqlPool) -> Result<()> {
        let mut orchestrator = Self::new(config, pool).await?;
        orchestrator.run().await
    }

    /// Create a new orchestrator instance
    async fn new(config: Config, pool: LibsqlPool) -> Result<Self> {
        let config = Arc::new(config);

        // Get database connection for initialization
        let conn = pool.get().await?;

        // Initialize database schema
        info!("Initializing database schema...");
        initialize_database(&conn).await?;

        // Create database instance with pool
        let database = Arc::new(DatabaseImpl::new_from_pool(pool));

        // Load or generate cryptographic keypair
        info!("Loading cryptographic keypair...");
        let keypair_path = PathBuf::from("uppe_keypair.key");
        let keypair = Arc::new(load_or_generate_keypair(&keypair_path)?);
        let peer_id = keypair.public_key_hex();
        info!("Peer ID (public key): {}", peer_id);

        // Create monitoring executor
        let executor = Arc::new(MonitoringExecutor::new(
            peer_id.clone(),
            config.preferences.timeout_seconds.unwrap_or(10),
            config.preferences.degraded_threshold_ms.unwrap_or(1000),
        )?);

        // Create P2P network
        let p2p_network =
            Arc::new(P2PNetwork::new(peer_id.clone(), config.preferences.use_peerup_layer));

        Ok(Self { config, database, keypair, executor, p2p_network, task_handles: Vec::new() })
    }

    /// Run the orchestrator
    async fn run(&mut self) -> Result<()> {
        info!("Starting Uppe orchestrator...");

        // Start P2P network if enabled
        if self.p2p_network.is_enabled() {
            info!("Starting P2P network...");
            self.p2p_network.start().await?;
        } else {
            info!("P2P network is disabled - running in isolated mode");
        }

        // Create channels for communication
        let (result_tx, mut result_rx) = mpsc::channel::<CheckResult>(100);

        // Create scheduler
        let scheduler = MonitoringScheduler::new(self.executor.clone(), result_tx.clone());

        // Load monitors from database and schedule them
        info!("Loading monitors from database...");
        let monitors = self.database.get_enabled_monitors().await?;
        info!("Found {} enabled monitors", monitors.len());

        // Convert database monitors to scheduler configs
        let monitor_configs: Vec<MonitorConfig> = monitors
            .into_iter()
            .map(|m| {
                let check_type = match m.check_type.as_str() {
                    "http" => CheckType::Http,
                    "https" => CheckType::Https,
                    "tcp" => CheckType::Tcp,
                    "icmp" => CheckType::Icmp,
                    _ => CheckType::Http,
                };

                MonitorConfig {
                    id: m.uuid,
                    target: m.target,
                    check_type,
                    interval_seconds: m.interval_seconds,
                    enabled: m.enabled,
                }
            })
            .collect();

        // Schedule all monitors
        info!("Scheduling monitors...");
        self.task_handles = scheduler.schedule_monitors(monitor_configs);

        // Process results in a loop
        info!("Orchestrator started successfully - processing monitoring results");

        // Track last location check
        let mut last_location_check = std::time::Instant::now();
        let location_check_interval = std::time::Duration::from_secs(60); // Check every minute if update is needed

        while let Some(result) = result_rx.recv().await {
            // Periodically check if location needs updating (for mobile devices)
            if last_location_check.elapsed() >= location_check_interval {
                crate::location::check_and_update_location();
                last_location_check = std::time::Instant::now();
            }

            // Sign the result
            let signature = sign_result(&result, &self.keypair)?;
            let signed_result = result.with_signature(signature);

            // Save to database
            if let Err(e) = self.database.save_result(&signed_result).await {
                error!("Failed to save result to database: {}", e);
            }

            // Share with P2P network if enabled
            if self.p2p_network.is_enabled() {
                if let Err(e) = self.p2p_network.share_result(&signed_result).await {
                    error!("Failed to share result with P2P network: {}", e);
                }
            }

            // Log the result
            info!(
                "Monitor {} - {} - Status: {} - Latency: {:?}ms",
                signed_result.monitor_id,
                signed_result.target,
                signed_result.status,
                signed_result.latency_ms
            );
        }

        Ok(())
    }
}
