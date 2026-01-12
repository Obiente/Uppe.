/// Orchestrator module - coordinates all components
///
/// The orchestrator is the core coordinator that:
/// - Manages the lifecycle of all components
/// - Coordinates between monitoring, database, crypto, and P2P layers
/// - Handles results and distributes them appropriately
use anyhow::Result;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::config::Config;
use crate::crypto::{KeyPair, load_or_generate_keypair, sign_result, verify_result};
use crate::database::{Database, DatabaseImpl, initialize_database};
use crate::database::models::{NetworkStats, Peer};
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
        let keypair_path = std::env::var("UPPE_KEYPAIR_PATH")
            .unwrap_or_else(|_| "uppe_keypair.key".to_string());
        let keypair_path = PathBuf::from(keypair_path);
        let keypair = Arc::new(load_or_generate_keypair(&keypair_path)?);
        let peer_id = keypair.public_key_hex();
        info!("Peer ID (public key): {}", peer_id);

        // Create monitoring executor
        let executor = Arc::new(MonitoringExecutor::new(
            peer_id.clone(),
            config.preferences.timeout_seconds.unwrap_or(10),
            config.preferences.degraded_threshold_ms.unwrap_or(1000),
        )?);

        // Create P2P network with configuration
        let mut builder = peerup::node::NodeConfig::builder()
            .port_range(config.peerup.port_range)
            .bootstrap_peers(config.peerup.bootstrap_peers.clone());

        // Conditionally enable/disable features
        if config.peerup.enable_mdns {
            builder = builder.enable_mdns();
        } else {
            builder = builder.disable_mdns();
        }

        if config.peerup.enable_kademlia {
            builder = builder.enable_kademlia();
        } else {
            builder = builder.disable_kademlia();
        }

        if config.peerup.enable_relay {
            builder = builder.enable_relay();
        } else {
            builder = builder.disable_relay();
        }

        let peerup_config = builder.build();

        let mut p2p_network = P2PNetwork::with_config(
            peer_id.clone(),
            config.preferences.use_peerup_layer,
            keypair.public_key_bytes(),
            peerup_config,
        );

        // Start P2P network if enabled
        if p2p_network.is_enabled() {
            info!("Starting P2P network...");
            p2p_network.start().await?;
        } else {
            info!("P2P network is disabled - running in isolated mode");
        }

        Ok(Self {
            config,
            database,
            keypair,
            executor,
            p2p_network: Arc::new(p2p_network),
            task_handles: Vec::new(),
        })
    }

    /// Run the orchestrator
    async fn run(&mut self) -> Result<()> {
        info!("Starting Uppe orchestrator...");

        // P2P network was already started in new(), no need to start again

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
        let mut last_location_check = Instant::now();
        let location_check_interval = Duration::from_secs(60); // Check every minute if update is needed

        // P2P/network stats tracking
        let mut connected_peers: HashSet<String> = HashSet::new();
        let mut total_peers_seen: HashSet<String> = HashSet::new();
        let mut checks_performed: i64 = 0;
        let mut checks_received: i64 = 0;
        let mut last_stats_persist = Instant::now();
        let stats_persist_interval = Duration::from_secs(5);

        // Get mutable reference to p2p_network for event handling
        let p2p_network = Arc::get_mut(&mut self.p2p_network)
            .expect("P2P network should not have multiple references at this point");

        loop {
            tokio::select! {
                // Handle monitoring results
                Some(result) = result_rx.recv() => {
                    // Periodically check if location needs updating (for mobile devices)
                    if last_location_check.elapsed() >= location_check_interval {
                        crate::location::check_and_update_location();
                        last_location_check = Instant::now();
                    }

                    // Sign the result
                    let signature = sign_result(&result, &self.keypair)?;
                    let signed_result = result.with_signature(signature);

                    // Save to database
                    if let Err(e) = self.database.save_result(&signed_result).await {
                        error!("Failed to save result to database: {}", e);
                    }

                    // Update stats for locally performed check
                    checks_performed += 1;

                    // Share with P2P network if enabled
                    if p2p_network.is_enabled() {
                        if let Err(e) = p2p_network.share_result(&signed_result).await {
                            error!("Failed to share result with P2P network: {}", e);
                        }
                    }

                    if last_stats_persist.elapsed() >= stats_persist_interval {
                        let snapshot = NetworkStats {
                            timestamp: SystemTime::now(),
                            total_peers: total_peers_seen.len() as i64,
                            online_peers: connected_peers.len() as i64,
                            checks_performed,
                            checks_received,
                            bandwidth_used_mb: 0,
                        };

                        if let Err(e) = self.database.insert_network_stats(&snapshot).await {
                            warn!("Failed to persist network stats: {}", e);
                        }

                        last_stats_persist = Instant::now();
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

                // Handle P2P events
                Some(p2p_event) = p2p_network.next_event() => {
                    use crate::p2p::P2PEvent;
                    match p2p_event {
                        P2PEvent::ResultReceived { peer_id, result } => {
                            info!("Received monitoring result from peer {}", peer_id);

                            total_peers_seen.insert(peer_id.clone());

                            // Convert P2P result to database model
                            if let Some(mut db_result) = crate::database::models::PeerResult::from_p2p_result(&result) {
                                // Verify signature if public key is available
                                let verified = if let Some(public_key_vec) = &result.public_key {
                                    if public_key_vec.len() == 32 {
                                        let mut public_key_bytes = [0u8; 32];
                                        public_key_bytes.copy_from_slice(&public_key_vec[..32]);
                                        
                                        // Verify the signature
                                        match verify_result(&db_result, &public_key_bytes, &result.result.target) {
                                            Ok(true) => {
                                                info!("Successfully verified signature from peer {}", peer_id);
                                                true
                                            }
                                            Ok(false) => {
                                                warn!("Invalid signature from peer {}", peer_id);
                                                false
                                            }
                                            Err(e) => {
                                                error!("Signature verification error from peer {}: {}", peer_id, e);
                                                false
                                            }
                                        }
                                    } else {
                                        warn!("Invalid public key length from peer {}: {} bytes", peer_id, public_key_vec.len());
                                        false
                                    }
                                } else {
                                    warn!("Received peer result without public key from {}", peer_id);
                                    false
                                };
                                
                                db_result.verified = verified;

                                // Keep peer record fresh when results arrive
                                let peer_model = Peer::new_online(peer_id.clone(), SystemTime::now());
                                if let Err(e) = self.database.upsert_peer(&peer_model).await {
                                    warn!("Failed to upsert peer {} on result: {}", peer_id, e);
                                }
                                
                                if let Err(e) = self.database.save_peer_result(&db_result).await {
                                    error!("Failed to save peer result: {}", e);
                                } else {
                                    let status = if verified { "verified" } else { "unverified" };
                                    debug!("Successfully saved {} peer result from {}", status, peer_id);
                                }

                                // Update stats for received results
                                checks_received += 1;
                            } else {
                                warn!("Received peer result without signature from {}", peer_id);
                            }
                        }
                        P2PEvent::PeerConnected(peer_id) => {
                            info!("Peer connected: {}", peer_id);

                            let now = SystemTime::now();
                            connected_peers.insert(peer_id.clone());
                            total_peers_seen.insert(peer_id.clone());

                            let peer_model = Peer::new_online(peer_id.clone(), now);
                            if let Err(e) = self.database.upsert_peer(&peer_model).await {
                                warn!("Failed to upsert peer {}: {}", peer_id, e);
                            }
                        }
                        P2PEvent::PeerDisconnected(peer_id) => {
                            info!("Peer disconnected: {}", peer_id);

                            connected_peers.remove(&peer_id);
                            if let Err(e) = self.database.mark_peer_offline(&peer_id, SystemTime::now()).await {
                                warn!("Failed to mark peer offline {}: {}", peer_id, e);
                            }
                        }
                        P2PEvent::Started { peer_id } => {
                            info!("P2P network started with peer ID: {}", peer_id);
                        }
                        P2PEvent::Error(err) => {
                            error!("P2P error: {}", err);
                        }
                        _ => {
                            tracing::trace!("P2P event: {:?}", p2p_event);
                        }
                    }

                    if last_stats_persist.elapsed() >= stats_persist_interval {
                        let snapshot = NetworkStats {
                            timestamp: SystemTime::now(),
                            total_peers: total_peers_seen.len() as i64,
                            online_peers: connected_peers.len() as i64,
                            checks_performed,
                            checks_received,
                            bandwidth_used_mb: 0,
                        };

                        if let Err(e) = self.database.insert_network_stats(&snapshot).await {
                            warn!("Failed to persist network stats: {}", e);
                        }

                        last_stats_persist = Instant::now();
                    }
                }

                else => {
                    info!("All channels closed, shutting down orchestrator");
                    break;
                }
            }
        }

        Ok(())
    }
}
