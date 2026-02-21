/// Orchestrator module - coordinates all components
///
/// The orchestrator is the core coordinator that:
/// - Manages the lifecycle of all components
/// - Coordinates between monitoring, database, crypto, and P2P layers
/// - Handles results and distributes them appropriately
///
/// ## Distributed Orchestration
/// The `distributed` submodule provides consensus-based orchestration for
/// public monitors, coordinating checks across peers to prevent abuse.
///
/// ## Private Monitor Orchestration
/// The `private` submodule provides peer-assisted monitoring with encryption
/// for private services, coordinating helper peers and result synchronization.

pub mod distributed;
pub mod private;
pub mod admission;
pub mod admin_trust;
pub mod retention;

#[cfg(test)]
mod tests;

pub use distributed::DistributedOrchestrator;
pub use private::PrivateMonitorOrchestrator;
pub use retention::{RetentionCleanup, RetentionPolicy};

use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::config::Config;
use crate::crypto::{KeyPair, load_or_generate_keypair, sign_result, verify_result, encrypt_result_for_owner};
use crate::database::models::{NetworkStats, Peer};
use crate::database::{Database, DatabaseImpl, initialize_database};
use crate::monitoring::checker::CheckType;
use crate::monitoring::scheduler::MonitorConfig;
use crate::monitoring::{CheckResult, MonitoringExecutor, MonitoringScheduler};
use crate::p2p::P2PNetwork;
use crate::pool::LibsqlPool;

/// Info about a helper assignment that this peer is helping with
#[derive(Debug, Clone)]
struct HelperAssignmentInfo {
    monitor_uuid: Uuid,
    owner_peer_id: String,
    owner_public_key: [u8; 32],
}

/// Main orchestrator for the Uppe service
pub struct Orchestrator {
    #[allow(dead_code)] // Will be used for runtime configuration changes
    config: Arc<Config>,
    database: Arc<dyn Database>,
    keypair: Arc<KeyPair>,
    executor: Arc<MonitoringExecutor>,
    p2p_network: Arc<P2PNetwork>,
    p2p_event_rx: Option<mpsc::Receiver<crate::p2p::P2PEvent>>,
    task_handles: Vec<tokio::task::JoinHandle<()>>,
    #[allow(dead_code)] // Background task handle kept alive
    retention_cleanup_handle: Option<tokio::task::JoinHandle<()>>,
    #[allow(dead_code)] // Kept alive to manage private monitor helper assignments
    private_orchestrator: Option<Arc<PrivateMonitorOrchestrator>>,
    /// Distributed orchestrator for public monitor consensus
    distributed_orchestrator: Option<Arc<DistributedOrchestrator>>,
    /// Tracks helper assignments this peer is helping with (monitor_uuid -> owner info)
    helper_assignments: Arc<tokio::sync::RwLock<HashMap<Uuid, HelperAssignmentInfo>>>,
    /// Rate limiter for private monitor assignments
    rate_limiter: Arc<tokio::sync::RwLock<private::PrivateMonitorRateLimiter>>,
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
        let keypair_path =
            std::env::var("UPPE_KEYPAIR_PATH").unwrap_or_else(|_| "uppe_keypair.key".to_string());
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

        // Start P2P network if enabled and store the event receiver
        let p2p_event_rx = if p2p_network.is_enabled() {
            info!("Starting P2P network...");
            Some(p2p_network.start().await?)
        } else {
            info!("P2P network is disabled - running in isolated mode");
            None
        };

        // Initialize retention cleanup with default policy
        info!("Starting retention cleanup background task...");
        let retention_policy = RetentionPolicy::default();
        info!(
            "Retention policy: private={}d, public={}d, peer={}d",
            retention_policy.private_result_days,
            retention_policy.public_result_days,
            retention_policy.peer_result_days
        );
        let retention_cleanup = RetentionCleanup::new(database.clone(), retention_policy);
        let retention_handle = retention_cleanup.start_periodic_cleanup();

        Ok(Self {
            config,
            database,
            keypair,
            executor,
            p2p_network: Arc::new(p2p_network),
            p2p_event_rx,
            task_handles: Vec::new(),
            retention_cleanup_handle: Some(retention_handle),
            private_orchestrator: None, // Will be set in run()
            distributed_orchestrator: None, // Will be set in run()
            helper_assignments: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            rate_limiter: Arc::new(tokio::sync::RwLock::new(private::PrivateMonitorRateLimiter::new())),
        })
    }

    /// Run the orchestrator
    async fn run(&mut self) -> Result<()> {
        info!("Starting Uppe orchestrator...");

        // P2P network was already started in new(), no need to start again

        // Create channels for communication
        let (result_tx, mut result_rx) = mpsc::channel::<CheckResult>(100);

        // Create scheduler (mutable for dynamic monitor reloading)
        let mut scheduler = MonitoringScheduler::new(self.executor.clone(), result_tx.clone());

        // Load monitors from database and schedule them
        info!("Loading monitors from database...");
        let monitors = self.database.get_enabled_monitors().await?;
        info!("Found {} enabled monitors", monitors.len());

        // Create monitor visibility map to check before sharing results (mutable for dynamic updates)
        let mut monitor_visibility: HashMap<Uuid, crate::database::models::MonitorVisibility> = HashMap::new();
        for m in &monitors {
            monitor_visibility.insert(m.uuid, m.visibility.clone());
        }

        // Run Owner Sync if enabled (attempts to sync encrypted private results from DHT)
        // This is optional and will log warnings on failure without stopping startup
        if self.p2p_network.is_enabled() {
            info!("Attempting Owner Sync from DHT...");
            let owner_secret_key = self.keypair.x25519_secret_bytes();
            let owner_pubkey = self.keypair.x25519_public_key();
            let private_orchestrator = PrivateMonitorOrchestrator::new(
                self.database.clone(),
                self.keypair.public_key_hex(),
                owner_pubkey,
                self.p2p_network.clone(),
            );

            // Initialize private monitor orchestrator - assign helpers for private monitors
            if let Err(e) = private_orchestrator.initialize().await {
                warn!("Failed to initialize private monitor orchestrator: {}", e);
            }

            match private_orchestrator.sync_owner_results_from_dht(&owner_secret_key).await {
                Ok(()) => info!("Owner sync completed successfully"),
                Err(e) => warn!("Owner sync failed (this is normal on first run): {}", e),
            }

            // Keep the orchestrator alive for dynamic helper assignment
            self.private_orchestrator = Some(Arc::new(private_orchestrator));

            // Initialize distributed orchestrator for public monitor consensus
            let distributed_orchestrator = DistributedOrchestrator::new(
                self.database.clone(),
                self.keypair.public_key_hex(),
                self.keypair.clone(),
                self.p2p_network.clone(),
            );
            if let Err(e) = distributed_orchestrator.initialize().await {
                warn!("Failed to initialize distributed orchestrator: {}", e);
            }
            self.distributed_orchestrator = Some(Arc::new(distributed_orchestrator));
        } else {
            info!("Skipping Owner Sync - P2P network disabled");
        }

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

        // Track last monitor reload check
        let mut last_monitor_reload = Instant::now();
        let monitor_reload_interval = Duration::from_secs(30); // Check for new monitors every 30 seconds

        // Track which private monitors have already been assigned helpers
        // This prevents reassignment churn on every monitor reload
        let mut assigned_private_monitors: HashSet<Uuid> = HashSet::new();

        // Track last helper health check
        let mut last_helper_maintenance = Instant::now();
        let helper_maintenance_interval = Duration::from_secs(60); // Check helper health every minute

        // P2P/network stats tracking
        let mut connected_peers: HashSet<String> = HashSet::new();
        let mut total_peers_seen: HashSet<String> = HashSet::new();
        let mut checks_performed: i64 = 0;
        let mut checks_received: i64 = 0;
        let mut last_stats_persist = Instant::now();
        let stats_persist_interval = Duration::from_secs(5);

        // Use the P2P event receiver that was returned from start()
        let mut p2p_event_rx = self.p2p_event_rx.take();
        let p2p_network = self.p2p_network.clone();
        // Subscribe to TUI bus for requests (e.g., DHT queries)
        let mut tui_rx = crate::tui::bus::subscribe();

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

                    // Check if this result is for a helper assignment (we're helping someone monitor)
                    let assignments = self.helper_assignments.read().await;
                    if let Some(assignment) = assignments.get(&signed_result.monitor_id) {
                        // This is a helper assignment - encrypt and send the result back to the owner
                        let assignment_clone = assignment.clone();
                        drop(assignments); // Release the lock
                        
                        info!("Encrypting helper result for monitor {} and owner {}", 
                              signed_result.monitor_id, assignment_clone.owner_peer_id);
                        
                        match encrypt_result_for_owner(
                            &signed_result,
                            &assignment_clone.owner_public_key,
                            self.keypair.public_key_hex(),
                            assignment_clone.owner_peer_id.clone(),
                            assignment_clone.monitor_uuid.to_string(),
                        ) {
                            Ok(encrypted_result) => {
                                // Send encrypted result back to owner via P2P
                                if let Err(e) = p2p_network.publish_encrypted_result(&encrypted_result).await {
                                    error!("Failed to publish encrypted result for {}: {}", signed_result.monitor_id, e);
                                } else {
                                    info!("Published encrypted result for {} to owner {}", signed_result.monitor_id, assignment_clone.owner_peer_id);
                                    checks_performed += 1;
                                }
                            }
                            Err(e) => {
                                error!("Failed to encrypt result for helper assignment: {}", e);
                            }
                        }
                        continue; // Don't save to our own database or share publicly
                    }
                    drop(assignments); // Release the lock if no assignment found

                    // Save to database
                    if let Err(e) = self.database.save_result(&signed_result).await {
                        error!("Failed to save result to database: {}", e);
                    }

                    // Update stats for locally performed check
                    checks_performed += 1;

                    // Share with P2P network ONLY if it's a PUBLIC monitor
                    // Private and Internal monitors should NEVER be shared via P2P gossipsub
                    if p2p_network.is_enabled() {
                        if let Some(visibility) = monitor_visibility.get(&signed_result.monitor_id) {
                            use crate::database::models::MonitorVisibility;
                            if matches!(visibility, MonitorVisibility::Public) {
                                if let Err(e) = p2p_network.share_result(&signed_result).await {
                                    error!("Failed to share public monitor result with P2P network: {}", e);
                                }
                            } else {
                                debug!("Skipping P2P share for {:?} monitor {}", visibility, signed_result.monitor_id);
                            }
                        } else {
                            warn!("Unknown monitor visibility for {}, not sharing to P2P", signed_result.monitor_id);
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
                Some(p2p_event) = async {
                    if let Some(ref mut rx) = p2p_event_rx {
                        rx.recv().await
                    } else {
                        std::future::pending().await
                    }
                }, if p2p_event_rx.is_some() => {
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
                                                debug!("Verified signature from peer {}", peer_id);
                                                true
                                            }
                                            Ok(false) => {
                                                warn!(
                                                    target: "uppe::audit",
                                                    peer = %peer_id,
                                                    monitor = %result.result.monitor_id,
                                                    "Signature verification failed for peer result"
                                                );
                                                false
                                            }
                                            Err(e) => {
                                                warn!(
                                                    target: "uppe::audit",
                                                    peer = %peer_id,
                                                    error = %e,
                                                    "Signature verification error"
                                                );
                                                false
                                            }
                                        }
                                    } else {
                                        warn!(
                                            target: "uppe::audit",
                                            peer = %peer_id,
                                            key_len = public_key_vec.len(),
                                            "Invalid public key length in peer result"
                                        );
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
                            info!(target: "uppe::audit", peer = %peer_id, "Peer connected");

                            let now = SystemTime::now();
                            connected_peers.insert(peer_id.clone());
                            total_peers_seen.insert(peer_id.clone());

                            // Publish live peers to TUI bus
                            #[allow(unused_must_use)]
                            {
                                crate::tui::bus::publish_peers(connected_peers.iter().cloned().collect(), now);
                            }

                            // Update private orchestrator with new peer
                            if let Some(private_orch) = &self.private_orchestrator {
                                private_orch.handle_peer_connected(peer_id.clone()).await;
                            }

                            let peer_model = Peer::new_online(peer_id.clone(), now);
                            if let Err(e) = self.database.upsert_peer(&peer_model).await {
                                warn!("Failed to upsert peer {}: {}", peer_id, e);
                            }
                        }
                        P2PEvent::PeerDisconnected(peer_id) => {
                            info!(target: "uppe::audit", peer = %peer_id, "Peer disconnected");

                            connected_peers.remove(&peer_id);
                            #[allow(unused_must_use)]
                            {
                                crate::tui::bus::publish_peers(connected_peers.iter().cloned().collect(), SystemTime::now());
                            }
                            
                            // Update private orchestrator with disconnected peer
                            if let Some(private_orch) = &self.private_orchestrator {
                                private_orch.handle_peer_disconnected(&peer_id).await;
                            }
                            
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
                        P2PEvent::HelperAssignmentRequested { from_peer, request } => {
                            info!(
                                "Received helper assignment from peer {} for monitor {} (target: {})",
                                from_peer, request.monitor_uuid, request.target
                            );

                            // Parse the UUID
                            if let Ok(monitor_id) = uuid::Uuid::parse_str(&request.monitor_uuid) {
                                // Check if we have capacity for more assignments
                                let current_assignments = self.helper_assignments.read().await.len();
                                let max_assignments = 10; // Configurable limit
                                
                                if current_assignments >= max_assignments {
                                    warn!("Rejecting helper assignment - at capacity ({}/{})", current_assignments, max_assignments);

                                    let rejection = crate::p2p::messages::HelperAssignmentResponse::Rejected {
                                        monitor_uuid: request.monitor_uuid.clone(),
                                        reason: format!("Helper at capacity ({}/{})", current_assignments, max_assignments),
                                    };

                                    if let Err(e) = p2p_network.send_command(
                                        crate::p2p::messages::P2PCommand::SendHelperResponse(rejection)
                                    ).await {
                                        error!("Failed to send helper rejection: {}", e);
                                    }
                                    continue;
                                }

                                // Rate limit check: prevent owner from requesting too many checks
                                {
                                    let mut limiter = self.rate_limiter.write().await;
                                    if !limiter.can_check(&request.owner_peer_id) {
                                        warn!(
                                            target: "uppe::audit",
                                            owner = %request.owner_peer_id,
                                            monitor = %request.monitor_uuid,
                                            "Rate limit exceeded for owner, rejecting helper assignment"
                                        );
                                        let rejection = crate::p2p::messages::HelperAssignmentResponse::Rejected {
                                            monitor_uuid: request.monitor_uuid.clone(),
                                            reason: "Owner rate limit exceeded".to_string(),
                                        };
                                        if let Err(e) = p2p_network.send_command(
                                            crate::p2p::messages::P2PCommand::SendHelperResponse(rejection)
                                        ).await {
                                            error!("Failed to send rate limit rejection: {}", e);
                                        }
                                        continue;
                                    }
                                }

                                // Store owner info for this assignment
                                let assignment_info = HelperAssignmentInfo {
                                    monitor_uuid: monitor_id,
                                    owner_peer_id: request.owner_peer_id.clone(),
                                    owner_public_key: request.owner_public_key,
                                };
                                
                                {
                                    let mut assignments = self.helper_assignments.write().await;
                                    assignments.insert(monitor_id, assignment_info);
                                }

                                // Start monitoring the assigned service
                                let check_type = match request.check_type.as_str() {
                                    "http" => CheckType::Http,
                                    "https" => CheckType::Https,
                                    "tcp" => CheckType::Tcp,
                                    "icmp" => CheckType::Icmp,
                                    _ => CheckType::Http,
                                };

                                let monitor_config = MonitorConfig {
                                    id: monitor_id,
                                    target: request.target.clone(),
                                    check_type,
                                    interval_seconds: request.interval_seconds,
                                    enabled: true,
                                };

                                // Schedule this helper monitor
                                let handle = scheduler.schedule_monitor(monitor_config);
                                self.task_handles.push(handle);

                                info!(
                                    "Successfully scheduled helper monitoring for {} (owner: {})",
                                    request.monitor_uuid, from_peer
                                );

                                // Send acceptance back to the owner
                                let acceptance = crate::p2p::messages::HelperAssignmentResponse::Accepted {
                                    monitor_uuid: request.monitor_uuid.clone(),
                                    helper_peer_id: self.keypair.public_key_hex(),
                                };
                                if let Err(e) = p2p_network.send_command(
                                    crate::p2p::messages::P2PCommand::SendHelperResponse(acceptance)
                                ).await {
                                    error!("Failed to send helper acceptance: {}", e);
                                }
                            } else {
                                warn!("Invalid monitor UUID in helper assignment: {}", request.monitor_uuid);
                            }
                        }
                        P2PEvent::EncryptedResultReceived { from_peer, result: encrypted_result } => {
                            info!(
                                "Received encrypted result from helper peer {} for monitor {}",
                                from_peer, encrypted_result.monitor_uuid
                            );

                            // Parse the monitor UUID
                            if let Ok(monitor_id) = uuid::Uuid::parse_str(&encrypted_result.monitor_uuid) {
                                // Update helper last_seen if we're the owner
                                if let Some(private_orch) = &self.private_orchestrator {
                                    private_orch.handle_helper_result(&from_peer).await;
                                }
                                
                                debug!(
                                    "Encrypted result from helper {} for owner {} (monitor {})",
                                    from_peer, encrypted_result.owner_peer_id, monitor_id
                                );
                                checks_received += 1;
                            } else {
                                warn!("Invalid monitor UUID in encrypted result: {}", encrypted_result.monitor_uuid);
                            }
                        }
                        P2PEvent::DhtSnapshot { snapshot } => {
                            // Persist latest snapshot and publish to TUI bus for live updates
                            match serde_json::to_string(&*snapshot) {
                                Ok(json) => {
                                    if let Err(e) = self.database.set_setting("dht_snapshot", &json).await {
                                        warn!("Failed to persist DHT snapshot: {}", e);
                                    }
                                }
                                Err(e) => warn!("Failed to serialize DHT snapshot: {}", e),
                            }
                            #[allow(unused_must_use)] { crate::tui::bus::publish_dht_snapshot((*snapshot).clone()); }
                        }
                        P2PEvent::DHTRecordReceived { key, record } => {
                            let key_str = String::from_utf8_lossy(&key).to_string();
                            #[allow(unused_must_use)] { crate::tui::bus::publish_dht_query_result(key_str, true, Some(record)); }
                        }
                        P2PEvent::DHTRecordNotFound { key } => {
                            let key_str = String::from_utf8_lossy(&key).to_string();
                            #[allow(unused_must_use)] { crate::tui::bus::publish_dht_query_result(key_str, false, None); }
                        }
                        P2PEvent::HelperAssignmentResponse { from_peer, response } => {
                            debug!(
                                "Received helper assignment response from peer {}",
                                from_peer
                            );
                            
                            // Route to private orchestrator
                            if let Some(private_orch) = &self.private_orchestrator {
                                use crate::p2p::messages::HelperAssignmentResponse;
                                match *response {
                                    HelperAssignmentResponse::Accepted { ref monitor_uuid, ref helper_peer_id } => {
                                        if let Err(e) = private_orch.handle_helper_accepted(monitor_uuid, helper_peer_id).await {
                                            warn!("Failed to handle helper acceptance: {}", e);
                                        }
                                    }
                                    HelperAssignmentResponse::Rejected { ref monitor_uuid, ref reason } => {
                                        if let Err(e) = private_orch.handle_helper_rejected(monitor_uuid, &from_peer, reason).await {
                                            warn!("Failed to handle helper rejection: {}", e);
                                        }
                                    }
                                }
                            }
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

                        // Publish live stats to TUI bus
                        #[allow(unused_must_use)]
                        {
                            crate::tui::bus::publish_network_stats(snapshot.clone());
                        }

                        last_stats_persist = Instant::now();
                    }
                }

                // Handle TUI bus requests (e.g., DHT queries)
                ev = tui_rx.recv() => {
                    if let Some(crate::tui::bus::TuiEvent::DhtQuery(key)) = ev.ok() {
                        info!(%key, "TUI bus: forwarding DHT GET to P2P network");
                        // Issue a DHT GET via P2P network
                        let _ = self.p2p_network.send_command(crate::p2p::messages::P2PCommand::GetDHTRecord { key: key.as_bytes().to_vec() }).await;
                    }
                }

                // Periodic task: Check for new or updated monitors
                _ = tokio::time::sleep_until(tokio::time::Instant::from_std(last_monitor_reload + monitor_reload_interval)) => {
                    if last_monitor_reload.elapsed() >= monitor_reload_interval {
                        debug!("Checking for new or updated monitors...");
                        
                        match self.database.get_enabled_monitors().await {
                            Ok(current_monitors) => {
                                // Build new monitor configs
                                let new_monitor_configs: Vec<MonitorConfig> = current_monitors
                                    .iter()
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
                                            target: m.target.clone(),
                                            check_type,
                                            interval_seconds: m.interval_seconds,
                                            enabled: m.enabled,
                                        }
                                    })
                                    .collect();

                                // Update monitor visibility map
                                monitor_visibility.clear();
                                for m in &current_monitors {
                                    monitor_visibility.insert(m.uuid, m.visibility.clone());
                                }

                                // Check for new private monitors and assign helpers
                                if let Some(private_orch) = &self.private_orchestrator {
                                    let private_monitors: Vec<_> = current_monitors
                                        .iter()
                                        .filter(|m| m.is_private())
                                        .collect();
                                    
                                    for monitor in private_monitors {
                                        // Only assign helpers if this is a NEW private monitor
                                        // Don't reassign on every reload - that causes unnecessary churn
                                        if !assigned_private_monitors.contains(&monitor.uuid) {
                                            match private_orch.handle_new_monitor(monitor).await {
                                                Ok(()) => {
                                                    debug!("Assigned helpers for new private monitor: {}", monitor.uuid);
                                                    assigned_private_monitors.insert(monitor.uuid);
                                                }
                                                Err(e) => {
                                                    warn!("Failed to assign helpers for monitor {}: {}", monitor.uuid, e);
                                                }
                                            }
                                        }
                                    }
                                }

                                // Remove deleted monitors from the tracking set
                                let current_uuids: HashSet<Uuid> = current_monitors.iter().map(|m| m.uuid).collect();
                                assigned_private_monitors.retain(|uuid| current_uuids.contains(uuid));

                                // Cancel old tasks and reschedule all monitors
                                for handle in self.task_handles.drain(..) {
                                    handle.abort();
                                }
                                
                                self.task_handles = scheduler.schedule_monitors(new_monitor_configs.clone());
                                
                                info!("Reloaded monitors: {} active", new_monitor_configs.len());
                            }
                            Err(e) => {
                                error!("Failed to reload monitors: {}", e);
                            }
                        }
                        last_monitor_reload = Instant::now();
                    }
                }

                // Periodic task: Helper health maintenance
                _ = tokio::time::sleep_until(tokio::time::Instant::from_std(last_helper_maintenance + helper_maintenance_interval)) => {
                    if last_helper_maintenance.elapsed() >= helper_maintenance_interval {
                        if let Some(private_orch) = &self.private_orchestrator {
                            debug!("Running helper health maintenance");
                            if let Err(e) = private_orch.run_maintenance().await {
                                warn!("Helper maintenance failed: {}", e);
                            }
                        }
                        last_helper_maintenance = Instant::now();
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
