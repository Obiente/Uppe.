pub mod admin_trust;
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
pub mod retention;

#[cfg(test)]
mod tests;

pub use distributed::DistributedOrchestrator;
pub use private::PrivateMonitorOrchestrator;
pub use retention::{RetentionCleanup, RetentionPolicy};

use anyhow::Result;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::config::Config;
use crate::crypto::{
    KeyPair, decrypt_result_for_owner, encrypt_result_for_owner, load_or_generate_keypair,
    sign_result, verify_result,
};
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
    expires_at: Instant,
}

/// Main orchestrator for the Uppe service
pub struct Orchestrator {
    #[allow(dead_code)] // Will be used for runtime configuration changes
    config: Arc<Config>,
    actor_id: String,
    database: Arc<dyn Database>,
    keypair: Arc<KeyPair>,
    executor: Arc<MonitoringExecutor>,
    p2p_network: Arc<P2PNetwork>,
    p2p_event_rx: Option<mpsc::Receiver<crate::p2p::P2PEvent>>,
    task_handles: HashMap<Uuid, tokio::task::JoinHandle<()>>,
    #[allow(dead_code)] // Background task handle kept alive
    retention_cleanup_handle: Option<tokio::task::JoinHandle<()>>,
    audit_handle: tokio::task::JoinHandle<()>,
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
        let audit_pool = pool.clone();
        let database = Arc::new(DatabaseImpl::new_from_pool(pool));

        // Load or generate cryptographic keypair
        info!("Loading cryptographic keypair...");
        let keypair_path = std::env::var("UPPE_KEYPAIR_PATH").unwrap_or_else(|_| {
            PathBuf::from(std::env::var("UPPE_DATA_DIR").unwrap_or_else(|_| ".uppe".into()))
                .join("node.key")
                .to_string_lossy()
                .into_owned()
        });
        let keypair_path = PathBuf::from(keypair_path);
        let keypair = Arc::new(load_or_generate_keypair(&keypair_path)?);
        let audit_key = keypair.clone();
        let audit_handle = tokio::spawn(async move {
            let mut timer = tokio::time::interval(Duration::from_secs(1));
            timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                timer.tick().await;
                match audit_pool.get().await {
                    Ok(conn) => {
                        if let Err(error) =
                            crate::database::audit_outbox::drain(&conn, &audit_key).await
                        {
                            tracing::error!(%error, "Audit queue retained for retry");
                        }
                    }
                    Err(error) => tracing::error!(%error, "Audit database unavailable"),
                }
            }
        });
        let peer_id = keypair.public_key_hex();
        database.set_setting("node_id", &peer_id).await?;
        info!("Peer ID (public key): {}", peer_id);

        // Create monitoring executor
        let executor = Arc::new(MonitoringExecutor::new(
            peer_id.clone(),
            config.preferences.timeout_seconds.unwrap_or(10),
            config.preferences.degraded_threshold_ms.unwrap_or(1000),
        )?);

        // Create P2P network with configuration
        // Derive a stable libp2p peer identity from the same Ed25519 secret
        // used for the app-layer peer ID, so the libp2p peer ID is persistent
        // across restarts and eliminates the root cause of the dual-identity
        // and self-healing issues.
        let mut builder = peerup::node::NodeConfig::builder()
            .port_range(config.peerup.port_range)
            .bootstrap_peers(config.peerup.bootstrap_peers.clone());

        {
            let mut secret = keypair.signing_key.to_bytes();
            builder = builder.identity_from_ed25519_secret(&mut secret)?;
        }

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
            actor_id: peer_id,
            database,
            keypair,
            executor,
            p2p_network: Arc::new(p2p_network),
            p2p_event_rx,
            task_handles: HashMap::new(),
            retention_cleanup_handle: Some(retention_handle),
            private_orchestrator: None,     // Will be set in run()
            distributed_orchestrator: None, // Will be set in run()
            audit_handle,
            helper_assignments: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            rate_limiter: Arc::new(tokio::sync::RwLock::new(
                private::PrivateMonitorRateLimiter::new(),
            )),
        })
    }

    /// Run the orchestrator
    async fn run(&mut self) -> Result<()> {
        info!("Starting Uppe orchestrator...");

        // P2P network was already started in new(), no need to start again

        // Create channels for communication
        let (result_tx, mut result_rx) = mpsc::channel::<CheckResult>(100);

        // Create scheduler (mutable for dynamic monitor reloading)
        let scheduler = MonitoringScheduler::new(self.executor.clone(), result_tx.clone());

        // Load monitors from database and schedule them
        info!("Loading monitors from database...");
        let monitors = self.database.get_enabled_monitors().await?;
        info!("Found {} enabled monitors", monitors.len());

        // Create monitor visibility map to check before sharing results (mutable for dynamic updates)
        let mut monitor_visibility: HashMap<Uuid, crate::database::models::MonitorVisibility> =
            HashMap::new();
        // Map: normalised target URL → local monitor UUID, for public monitors.
        // Used to remap incoming gossipsub results from other peers (which carry
        // their own UUIDs) to our local monitor UUID so the frontend can
        // aggregate results and peer counts correctly.
        let mut target_to_local_uuid: HashMap<String, Uuid> = HashMap::new();
        for m in &monitors {
            monitor_visibility.insert(m.uuid, m.visibility.clone());
            if m.is_public() {
                target_to_local_uuid.insert(m.target.trim_end_matches('/').to_lowercase(), m.uuid);
            }
        }

        // Run Owner Sync if enabled (attempts to sync encrypted private results from DHT)
        // This is optional and will log warnings on failure without stopping startup
        if self.p2p_network.is_enabled() && self.config.preferences.enable_distributed_monitoring {
            info!("Initializing experimental peer coordination");
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

            // NOTE: DHT sync is deferred to the first PeerConnected event so that
            // routing peers are available before we issue Kademlia queries.

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

        // Convert database monitors to scheduler configs (with staggered phase offsets)
        let my_peer_id = self.keypair.public_key_hex();
        let mut monitor_configs: Vec<MonitorConfig> = Vec::with_capacity(monitors.len());
        for m in monitors {
            let check_type = match m.check_type.to_lowercase().as_str() {
                "http" => CheckType::Http,
                "https" => CheckType::Https,
                "tcp" => CheckType::Tcp,
                "icmp" => CheckType::Icmp,
                _ => CheckType::Http,
            };
            let phase_offset_secs = if let (Some(orch), Some(domain)) =
                (&self.distributed_orchestrator, m.public_domain.as_deref())
            {
                orch.get_peer_phase_offset(domain, &my_peer_id).await
            } else {
                0
            };
            monitor_configs.push(MonitorConfig {
                id: m.uuid,
                target: m.target,
                check_type,
                interval_seconds: m.interval_seconds,
                timeout_seconds: m.timeout_seconds,
                allow_private_targets: matches!(
                    m.visibility,
                    crate::database::models::MonitorVisibility::Internal
                ),
                enabled: m.enabled,
                phase_offset_secs,
            });
        }

        let mut running_configs: HashMap<Uuid, MonitorConfig> =
            monitor_configs.iter().map(|c| (c.id, c.clone())).collect();
        // Schedule all monitors
        info!("Scheduling monitors...");
        self.task_handles = monitor_configs
            .into_iter()
            .map(|c| (c.id, scheduler.schedule_monitor(c)))
            .collect();

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

        // Track last reconnect attempt (for bootstrap peer re-dial).
        // On startup we retry with short delays so helper assignment doesn't
        // have to wait 30 s before a first peer is discovered.
        // Schedule: 5 s → 10 s → 20 s → 30 s (steady-state).
        let reconnect_interval = Duration::from_secs(30);
        let mut startup_reconnect_delays: VecDeque<Duration> = VecDeque::from([
            Duration::from_secs(5),
            Duration::from_secs(10),
            Duration::from_secs(20),
        ]);
        let mut current_reconnect_delay =
            startup_reconnect_delays.pop_front().unwrap_or(reconnect_interval);
        let mut last_reconnect_attempt = Instant::now();
        // Periodic presence re-announce: even without new connections, re-broadcast
        // so any peer that came up while we were already running can discover us.
        let mut last_periodic_announce = Instant::now();
        let periodic_announce_interval = Duration::from_secs(300); // every 5 min

        // P2P/network stats tracking
        let mut connected_peers: HashSet<String> = HashSet::new();
        let mut total_peers_seen: HashSet<String> = HashSet::new();
        // Only peers that have announced themselves as uppe-service nodes are eligible
        // as helper candidates.  Plain peerup routing/relay nodes are excluded.
        let mut uppe_service_peers: HashSet<String> = HashSet::new();
        let mut checks_performed: i64 = 0;
        let mut checks_received: i64 = 0;
        let mut last_stats_persist = Instant::now();
        let stats_persist_interval = Duration::from_secs(5);

        // Deferred DHT sync — triggered once on first peer connection.
        let mut dht_sync_done = false;

        // Name consensus: domain → { name → vote_count }
        let mut domain_name_votes: std::collections::HashMap<
            String,
            std::collections::HashMap<String, usize>,
        > = std::collections::HashMap::new();

        // Use the P2P event receiver that was returned from start()
        let mut p2p_event_rx = self.p2p_event_rx.take();
        let p2p_network = self.p2p_network.clone();
        // Subscribe to TUI bus for requests (e.g., DHT queries)
        let mut tui_rx = crate::tui::bus::subscribe();
        crate::tui::bus::set_backend_running(true);

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
                    } else if let Err(e) = crate::audit::record_local_result_event(
                        self.database.as_ref(),
                        self.keypair.as_ref(),
                        &self.actor_id,
                        "local_scheduler",
                        &signed_result,
                    )
                    .await
                    {
                        warn!("Failed to record local result audit event: {}", e);
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

                                if !verified || result.result.timestamp.duration_since(SystemTime::now()).is_ok_and(|age| age > Duration::from_secs(30)) || SystemTime::now().duration_since(result.result.timestamp).is_ok_and(|age| age > Duration::from_secs(300)) {
                                    warn!("Rejected invalid or stale peer result from {}", peer_id);
                                    continue;
                                }
                                db_result.verified = true;

                                let subscribed = self.database.get_monitor_by_uuid(db_result.monitor_uuid).await.ok().flatten()
                                    .is_some_and(|m| m.enabled && m.is_public() && m.target == result.result.target);
                                if !subscribed { continue; }
                                let receipt = crate::audit::peer_result_receipt(&self.keypair, &self.actor_id, "p2p_gossipsub", &db_result, &result.result.target)?;
                                if let Err(e) = self.database.save_peer_result(&db_result, &result.result.target, &receipt).await {
                                    warn!("Failed to save peer observation: {}", e);
                                }

                            } else {
                                warn!("Received peer result without signature from {}", peer_id);
                            }
                        }
                        P2PEvent::NodeAnnounced { libp2p_peer_id, app_peer_id: _ } => {
                            let is_new = !uppe_service_peers.contains(&libp2p_peer_id);
                            if is_new {
                                info!(peer = %libp2p_peer_id, "uppe-service peer announced");
                                if uppe_service_peers.len() >= 1024 { uppe_service_peers.retain(|p| connected_peers.contains(p)); }
                                if uppe_service_peers.len() < 1024 { uppe_service_peers.insert(libp2p_peer_id.clone()); }
                            }
                            // Update the private orchestrator's allowed helper set
                            if let Some(private_orch) = &self.private_orchestrator {
                                private_orch.update_uppe_service_peers(uppe_service_peers.clone()).await;
                                // Retry any monitors that previously had no helpers —
                                // a newly announced peer may now be eligible.
                                if is_new
                                    && let Err(e) = private_orch.retry_unhelped_monitors().await {
                                        tracing::warn!("retry_unhelped after announce failed: {}", e);
                                    }
                            }
                            // Broadcast all our public monitor groups so this new peer can
                            // discover and auto-join them.
                            if is_new
                                && let Some(dist_orch) = &self.distributed_orchestrator
                                    && let Err(e) = dist_orch.broadcast_all_for_discovery().await {
                                        tracing::debug!("broadcast_all_for_discovery failed: {}", e);
                                    }
                            // Mutual re-announce: respond with our own presence so the announcing
                            // peer learns we are also a uppe-service node.  Gossipsub does not
                            // replay history — the new peer subscribed to /uppe/announce/v1 AFTER
                            // we last published, so they never received our announcement.
                            if is_new && p2p_network.is_enabled() {
                                let _ = p2p_network.send_command(
                                    crate::p2p::messages::P2PCommand::AnnouncePresence
                                ).await;
                            }
                        }
                        P2PEvent::PeerConnected(peer_id) => {
                            info!(target: "uppe::audit", peer = %peer_id, "Peer connected");

                            let now = SystemTime::now();
                            connected_peers.insert(peer_id.clone());
                            if total_peers_seen.len() >= 4096 { total_peers_seen.clear(); }
                            total_peers_seen.insert(peer_id.clone());

                            // Publish live peers to TUI bus
                            #[allow(unused_must_use)]
                            {
                                crate::tui::bus::publish_peers(connected_peers.iter().cloned().collect(), now);
                            }

                            // Update private orchestrator with new peer
                            if let Some(private_orch) = &self.private_orchestrator {
                                private_orch.handle_peer_connected(peer_id.clone()).await;

                                // Clear private monitors from the "already assigned" set
                                // if they have no active/pending helpers.
                                let active = private_orch.get_assigned_monitor_uuids().await;
                                assigned_private_monitors.retain(|uuid| active.contains(uuid));

                                // Immediately retry any unhelped monitors now that a new peer
                                // is online — don't wait for the 30-second reload tick.
                                if let Err(e) = private_orch.retry_unhelped_monitors().await {
                                    tracing::warn!("retry_unhelped after peer connect failed: {}", e);
                                }
                            }

                            // Re-broadcast our own announcement so this peer learns we are uppe-service
                            if p2p_network.is_enabled() {
                                let _ = p2p_network.send_command(
                                    crate::p2p::messages::P2PCommand::AnnouncePresence
                                ).await;
                            }

                            let peer_model = Peer::new_online(peer_id.clone(), now);
                            if let Err(e) = self.database.upsert_peer(&peer_model).await {
                                warn!("Failed to upsert peer {}: {}", peer_id, e);
                            }

                            // First peer connection: trigger deferred DHT result sync.
                            // We spawn it so the event loop stays responsive.
                            if !dht_sync_done {
                                dht_sync_done = true;
                                if let Some(private_orch) = &self.private_orchestrator {
                                    let orch = Arc::clone(private_orch);
                                    let key = self.keypair.x25519_secret_bytes();
                                    tokio::spawn(async move {
                                        // Wait a moment for DHT routing to stabilise
                                        tokio::time::sleep(Duration::from_secs(4)).await;
                                        match orch.sync_owner_results_from_dht(&key).await {
                                            Ok(()) => info!("Deferred DHT sync completed"),
                                            Err(e) => warn!("Deferred DHT sync failed: {}", e),
                                        }
                                    });
                                }
                            }
                        }
                        P2PEvent::PeerDisconnected(peer_id) => {
                            info!(target: "uppe::audit", peer = %peer_id, "Peer disconnected");

                            connected_peers.remove(&peer_id);
                            uppe_service_peers.remove(&peer_id);
                            // Re-arm DHT sync so the next reconnect after a full
                            // network partition triggers encrypted-result recovery.
                            if connected_peers.is_empty() {
                                dht_sync_done = false;
                            }
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

                            // Remove disconnected peer from all public monitor groups
                            if let Some(dist_orch) = &self.distributed_orchestrator {
                                for group in dist_orch.get_all_groups().await {
                                    if group.participating_peers.contains(&peer_id)
                                        && let Err(e) = dist_orch.handle_peer_leave(group.domain.clone(), peer_id.clone()).await {
                                            tracing::debug!("handle_peer_leave {}/{}: {}", group.domain, peer_id, e);
                                        }
                                }
                            }
                        }
                        P2PEvent::Started { peer_id } => {
                            info!("P2P network started with peer ID: {}", peer_id);
                        }
                        P2PEvent::Error(err) => {
                            if err.contains("Duplicate") || err.contains("NoPeersSubscribedToTopic") {
                                tracing::debug!("P2P publish skipped (expected): {}", err);
                            } else {
                                error!("P2P error: {}", err);
                            }
                        }
                        P2PEvent::HelperAssignmentRequested { from_peer, request } => {
                            if !self.config.preferences.accept_remote_checks || request.interval_seconds < 10 || request.interval_seconds > 86400 || request.target.len() > 2048 || self.task_handles.contains_key(&Uuid::parse_str(&request.monitor_uuid).unwrap_or_default()) {
                                let _ = p2p_network.send_command(crate::p2p::messages::P2PCommand::SendHelperResponse(crate::p2p::messages::HelperAssignmentResponse::Rejected { monitor_uuid:request.monitor_uuid.clone(), reason:"Remote check is not permitted".into(), owner_libp2p_peer_id:request.owner_libp2p_peer_id.clone() })).await;
                                continue;
                            }
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
                                        owner_libp2p_peer_id: request.owner_libp2p_peer_id.clone(),
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
                                            owner_libp2p_peer_id: request.owner_libp2p_peer_id.clone(),
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
                                    expires_at: Instant::now() + Duration::from_secs(600),
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
                                    interval_seconds: request.interval_seconds.max(10), timeout_seconds: 10, allow_private_targets: false,
                                    enabled: true,
                                    phase_offset_secs: request.check_offset_seconds,
                                };

                                // Schedule this helper monitor
                                let handle = scheduler.schedule_monitor(monitor_config);
                                self.task_handles.insert(monitor_id, handle);

                                info!(
                                    "Successfully scheduled helper monitoring for {} (owner: {})",
                                    request.monitor_uuid, from_peer
                                );

                                // Send acceptance back to the owner
                                let acceptance = crate::p2p::messages::HelperAssignmentResponse::Accepted {
                                    monitor_uuid: request.monitor_uuid.clone(),
                                    helper_peer_id: self.keypair.public_key_hex(),
                                    owner_libp2p_peer_id: request.owner_libp2p_peer_id.clone(),
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
                                // If this node is a helper for this monitor (not the owner),
                                // the gossipsub message was looped back to us — ignore it.
                                let we_are_helper = self.helper_assignments.read().await.contains_key(&monitor_id);
                                if we_are_helper {
                                    debug!(
                                        monitor = %monitor_id,
                                        "Ignoring looped-back encrypted result (we are the helper, not the owner)"
                                    );
                                    // skip — don't try to decrypt with our own key
                                }
                                else {

                                // Decrypt and save the result to our local database
                                let owner_secret_key = self.keypair.x25519_secret_bytes();
                                match decrypt_result_for_owner::<CheckResult>(
                                    &encrypted_result,
                                    &owner_secret_key,
                                ) {
                                    Ok(decrypted) => {
                                        let assigned = if let Some(orch) = &self.private_orchestrator { orch.is_assigned_helper(&encrypted_result.monitor_uuid, &from_peer).await } else { false };
                                        let key = hex::decode(&decrypted.peer_id).unwrap_or_default();
                                        let Ok(key): Result<[u8;32],_> = key.try_into() else { continue; };
                                        let Some(signature) = decrypted.signature.clone() else { continue; };
                                        let observation = crate::database::models::PeerResult {
                                            id:None, monitor_uuid:decrypted.monitor_id, timestamp:decrypted.timestamp, status:decrypted.status,
                                            latency_ms:decrypted.latency_ms, status_code:decrypted.status_code, error_message:decrypted.error_message.clone(),
                                            peer_id:decrypted.peer_id.clone(), signature, verified:true, created_at:SystemTime::now(),
                                            city:None,country:None,region:None,source_peer_id:Some(from_peer.clone()),synced_from_peer:false,retention_until:None,
                                        };
                                        if !assigned || decrypted.monitor_id != monitor_id || encrypted_result.owner_peer_id != self.actor_id
                                            || peerup::crypto::transport_peer_id(&key).ok().as_deref() != Some(&from_peer)
                                            || !verify_result(&observation,&key,&decrypted.target).unwrap_or(false)
                                            || SystemTime::now().duration_since(decrypted.timestamp).map_or(true, |age| age > Duration::from_secs(300)) { continue; }
                                        let receipt = crate::audit::peer_result_receipt(&self.keypair, &self.actor_id, "helper_decrypted_result", &observation, &decrypted.target)?;
                                        match self.database.save_peer_result(&observation, &decrypted.target, &receipt).await {
                                            Ok(crate::database::peer_storage::SaveOutcome::Inserted) => {
                                                if let Some(orch) = &self.private_orchestrator { orch.handle_helper_result(&from_peer).await; }
                                                checks_received += 1;
                                            }
                                            Ok(_) => {},
                                            Err(e) => warn!("Failed to save helper observation: {}", e),
                                        }

                                    }
                                    Err(e) => {
                                        warn!(
                                            monitor = %monitor_id,
                                            from = %from_peer,
                                            "Failed to decrypt helper result: {}",
                                            e
                                        );
                                    }
                                }
                                } // end else (we are not the helper)
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
                                    HelperAssignmentResponse::Accepted { ref monitor_uuid, .. } => {
                                        // Use the libp2p peer ID (from_peer) rather than the
                                        // app peer ID from the response payload.  All other
                                        // helper tracking (uppe_service_peers, assignments,
                                        // handle_helper_result) uses libp2p IDs, so we must
                                        // be consistent here to avoid stale-helper false alarms.
                                        if let Err(e) = private_orch.handle_helper_accepted(monitor_uuid, &from_peer).await {
                                            warn!("Failed to handle helper acceptance: {}", e);
                                        }
                                    }
                                    HelperAssignmentResponse::Rejected { ref monitor_uuid, ref reason, .. } => {
                                        if let Err(e) = private_orch.handle_helper_rejected(monitor_uuid, &from_peer, reason).await {
                                            warn!("Failed to handle helper rejection: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                        P2PEvent::PublicMonitorAnnounced {
                            domain,
                            display_name,
                            creator_peer_id,
                            target,
                            check_type,
                            interval_seconds,
                        } => {
                            if !self.config.preferences.accept_remote_checks { continue; }
                            // Skip announcements originating from ourselves so we don't
                            // accidentally count our own broadcast as a name-consensus vote
                            // or try to re-join a group we already own.
                            let our_peer_id = self.keypair.public_key_hex();
                            if creator_peer_id == our_peer_id {
                                // nothing to do — we already have this monitor
                            } else {

                            // --- Name consensus: accumulate votes for this domain's display name ---
                            {
                                let votes = domain_name_votes
                                    .entry(domain.clone())
                                    .or_default();
                                *votes.entry(display_name.clone()).or_insert(0) += 1;

                                // Find the name with the most votes
                                if let Some((leading_name, &leading_count)) =
                                    votes.iter().max_by_key(|(_, c)| *c)
                                {
                                    // Only promote a new name when it has a clear majority
                                    // (≥ 2 votes ahead of any runner-up, and at least 2 votes)
                                    let runner_up = votes
                                        .iter()
                                        .filter(|(n, _)| *n != leading_name)
                                        .map(|(_, c)| *c)
                                        .max()
                                        .unwrap_or(0);
                                    if leading_count >= 2 && leading_count > runner_up + 1 {
                                        // Update stored display name if it differs
                                        let monitor_uuid = crate::database::models::Monitor::uuid_for_public_domain(&domain);
                                        if let Ok(Some(mut existing)) =
                                            self.database.get_monitor_by_uuid(monitor_uuid).await
                                            && existing.public_display_name.as_deref() != Some(leading_name.as_str()) {
                                                info!(
                                                    domain = %domain,
                                                    old_name = ?existing.public_display_name,
                                                    new_name = %leading_name,
                                                    votes = leading_count,
                                                    "Updating public monitor display name via consensus"
                                                );
                                                existing.public_display_name = Some(leading_name.clone());
                                                if let Err(e) = self.database.save_monitor(&existing).await {
                                                    warn!("Failed to update consensus display name for {}: {}", domain, e);
                                                } else if let Err(e) = crate::audit::record_monitor_event(
                                                    self.database.as_ref(),
                                                    self.keypair.as_ref(),
                                                    &self.actor_id,
                                                    "updated",
                                                    &existing,
                                                )
                                                .await
                                                {
                                                    warn!("Failed to record consensus monitor update audit event: {}", e);
                                                }
                                            }
                                    }
                                }
                            }

                            if let Some(dist_orch) = &self.distributed_orchestrator {
                                // Record announcing peer in the group + cast their schedule vote
                                if let Err(e) = dist_orch.handle_peer_join(domain.clone(), creator_peer_id.clone()).await {
                                    tracing::debug!("handle_peer_join {}/{}: {}", domain, creator_peer_id, e);
                                }
                                let vote = peerup::distributed::OrchestrationVote {
                                    domain: domain.clone(),
                                    schedule: peerup::distributed::OrchestrationSchedule {
                                        interval_seconds,
                                        assignments: Vec::new(),
                                    },
                                    voter_peer_id: creator_peer_id.clone(),
                                    signature: String::new(),
                                    public_key: None,
                                    timestamp: chrono::Utc::now().timestamp(),
                                };
                                if let Err(e) = dist_orch.handle_vote(vote).await {
                                    tracing::debug!("handle_vote {}: {}", domain, e);
                                }

                                // Only join if we don't already track this domain group
                                if dist_orch.get_group(&domain).await.is_none() {
                                    info!(
                                        domain = %domain,
                                        target = %target,
                                        creator = %creator_peer_id,
                                        "Auto-joining public monitor group"
                                    );

                                    // Build a new public monitor record so this node
                                    // can independently check the target.
                                    let mut monitor = crate::database::models::Monitor::new_public(
                                        format!("{} (community)", display_name),
                                        target.clone(),
                                        check_type.clone(),
                                        domain.clone(),
                                        display_name.clone(),
                                    );
                                    monitor.interval_seconds = interval_seconds;

                                    match self.database.save_monitor(&monitor).await {
                                        Ok(_) => {
                                            if let Err(e) = crate::audit::record_monitor_event(
                                                self.database.as_ref(),
                                                self.keypair.as_ref(),
                                                &self.actor_id,
                                                "created",
                                                &monitor,
                                            )
                                            .await
                                            {
                                                warn!("Failed to record auto-joined monitor audit event: {}", e);
                                            }
                                            if let Err(e) = dist_orch.handle_new_monitor(&monitor).await {
                                                warn!(
                                                    domain = %domain,
                                                    "Failed to join public monitor group: {}",
                                                    e
                                                );
                                            } else {
                                                info!(
                                                    domain = %domain,
                                                    target = %target,
                                                    "Joined public monitor group — will be scheduled in next reload tick"
                                                );
                                                // Trigger an immediate monitor reload so the
                                                // new entry is scheduled without waiting 30 s.
                                                last_monitor_reload =
                                                    Instant::now()
                                                        .checked_sub(monitor_reload_interval)
                                                        .unwrap_or_else(Instant::now);
                                            }
                                        }
                                        Err(e) => {
                                            warn!(
                                                domain = %domain,
                                                "Failed to save auto-joined public monitor: {}",
                                                e
                                            );
                                        }
                                    }
                                }
                            }
                            } // end else (creator_peer_id != our_peer_id)
                        }
                        P2PEvent::PublicMonitorGroupMessage { _from_peer: _, domain, message } => {
                            if let Some(dist_orch) = &self.distributed_orchestrator {
                                use peerup::distributed::PublicMonitorMessage;
                                match *message {
                                    PublicMonitorMessage::Join { peer_id, .. } => {
                                        if let Err(e) = dist_orch.handle_peer_join(domain.clone(), peer_id.clone()).await {
                                            tracing::debug!("handle_peer_join {}/{}: {}", domain, peer_id, e);
                                        }
                                    }
                                    PublicMonitorMessage::Leave { peer_id, .. } => {
                                        if let Err(e) = dist_orch.handle_peer_leave(domain.clone(), peer_id.clone()).await {
                                            tracing::debug!("handle_peer_leave {}/{}: {}", domain, peer_id, e);
                                        }
                                    }
                                    PublicMonitorMessage::Vote { vote } => {
                                        if let Err(error) = dist_orch.handle_vote(*vote).await { tracing::debug!(%error, "Rejected schedule vote"); }
                                    }                                    _ => {} // Announce covered by PublicMonitorAnnounced on discovery topic
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
                    if let Ok(crate::tui::bus::TuiEvent::DhtQuery(key)) = ev {
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
                                let my_peer_id = self.keypair.public_key_hex();
                                let mut new_monitor_configs: Vec<MonitorConfig> = Vec::with_capacity(current_monitors.len());
                                for m in &current_monitors {
                                    let check_type = match m.check_type.to_lowercase().as_str() {
                                        "http" => CheckType::Http,
                                        "https" => CheckType::Https,
                                        "tcp" => CheckType::Tcp,
                                        "icmp" => CheckType::Icmp,
                                        _ => CheckType::Http,
                                    };
                                    let phase_offset_secs = if let (Some(orch), Some(domain)) =
                                        (&self.distributed_orchestrator, m.public_domain.as_deref())
                                    {
                                        orch.get_peer_phase_offset(domain, &my_peer_id).await
                                    } else {
                                        0
                                    };
                                    new_monitor_configs.push(MonitorConfig {
                                        id: m.uuid,
                                        target: m.target.clone(),
                                        check_type,
                                        interval_seconds: m.interval_seconds, timeout_seconds: m.timeout_seconds, allow_private_targets: matches!(m.visibility, crate::database::models::MonitorVisibility::Internal),
                                        enabled: m.enabled,
                                        phase_offset_secs,
                                    });
                                }

                                // Update monitor visibility map
                                monitor_visibility.clear();
                                target_to_local_uuid.clear();
                                for m in &current_monitors {
                                    monitor_visibility.insert(m.uuid, m.visibility.clone());
                                    if m.is_public() {
                                        target_to_local_uuid.insert(
                                            m.target.trim_end_matches('/').to_lowercase(),
                                            m.uuid,
                                        );
                                    }
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

                                // Only start tasks for new monitors; abort tasks for removed ones.
                                // Existing running tasks are left untouched so their timers aren't reset.
                                let new_config_map: HashMap<Uuid, MonitorConfig> = new_monitor_configs
                                    .iter()
                                    .map(|c| (c.id, c.clone()))
                                    .collect();
                                let new_uuids: HashSet<Uuid> = new_config_map.keys().copied().collect();

                                // Collect helper-assigned UUIDs so their task handles
                                // are NOT aborted during reload.  Helper-assigned monitors
                                // live only in the owner's DB; aborting them here would
                                // silently stop helper peers from checking private monitors.
                                self.helper_assignments.write().await.retain(|_, assignment| assignment.expires_at > Instant::now());
                                let helper_assigned_uuids: HashSet<Uuid> = self
                                    .helper_assignments
                                    .read()
                                    .await
                                    .keys()
                                    .copied()
                                    .collect();

                                // Abort handles for monitors that no longer exist,
                                // but keep handles for active helper assignments.
                                self.task_handles.retain(|uuid, handle| {
                                    if !helper_assigned_uuids.contains(uuid) && (!new_uuids.contains(uuid) || running_configs.get(uuid) != new_config_map.get(uuid) || handle.is_finished()) {
                                        handle.abort();
                                        false
                                    } else {
                                        true
                                    }
                                });

                                // Schedule tasks for monitors not yet running
                                let currently_running: HashSet<Uuid> =
                                    self.task_handles.keys().copied().collect();
                                for uuid in new_uuids.difference(&currently_running) {
                                    if let Some(config) = new_config_map.get(uuid) {
                                        let handle = scheduler.schedule_monitor(config.clone());
                                        self.task_handles.insert(*uuid, handle);
                                        info!("Scheduled new monitor: {}", uuid);
                                    }
                                }

                                running_configs = new_config_map;
                                info!("Monitors active: {}", self.task_handles.len());
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
                            // After maintenance, clear assignment tracking for monitors
                            // that ended up with no helpers so the reload loop retries.
                            let active = private_orch.get_assigned_monitor_uuids().await;
                            assigned_private_monitors.retain(|uuid| active.contains(uuid));
                        }
                        last_helper_maintenance = Instant::now();
                    }
                }

                // Periodic task: Re-dial bootstrap peers when peer count is low.
                // Uses a startup backoff schedule (5 s → 10 s → 20 s) before settling
                // at the normal 30 s interval so the first peer is found quickly.
                _ = tokio::time::sleep_until(tokio::time::Instant::from_std(last_reconnect_attempt + current_reconnect_delay)) => {
                    if last_reconnect_attempt.elapsed() >= current_reconnect_delay {
                        // Re-dial when either we have very few connections OR we have
                        // connections but none have announced as uppe-service peers.
                        if p2p_network.is_enabled()
                            && (connected_peers.len() < 2 || uppe_service_peers.is_empty())
                        {
                            debug!(
                                "Peer count low ({}), re-dialing bootstrap peers (delay={}s)",
                                connected_peers.len(),
                                current_reconnect_delay.as_secs()
                            );
                            if let Err(e) = p2p_network.send_command(crate::p2p::messages::P2PCommand::DialBootstrapPeers).await {
                                warn!("Failed to send DialBootstrapPeers: {}", e);
                            }

                            // If we still have no uppe-service peers, immediately retry
                            // unhelped monitors — a peer that connected before it announced
                            // itself may now be eligible.
                            if uppe_service_peers.is_empty()
                                && let Some(private_orch) = &self.private_orchestrator
                                    && let Err(e) = private_orch.retry_unhelped_monitors().await {
                                        tracing::debug!("startup retry_unhelped: {}", e);
                                    }
                        }

                        // Advance through the startup schedule; stay at normal interval once exhausted.
                        current_reconnect_delay = startup_reconnect_delays
                            .pop_front()
                            .unwrap_or(reconnect_interval);
                        last_reconnect_attempt = Instant::now();
                    }
                }

                // Periodic: re-broadcast our node announcement so peers that came
                // online after our last connection-triggered announce can find us.
                _ = tokio::time::sleep_until(tokio::time::Instant::from_std(
                    last_periodic_announce + periodic_announce_interval
                )) => {
                    if last_periodic_announce.elapsed() >= periodic_announce_interval
                        && p2p_network.is_enabled()
                    {
                        let _ = p2p_network.send_command(
                            crate::p2p::messages::P2PCommand::AnnouncePresence
                        ).await;
                        last_periodic_announce = Instant::now();
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

impl Drop for Orchestrator {
    fn drop(&mut self) {
        crate::tui::bus::set_backend_running(false);
        self.audit_handle.abort();
        if let Some(handle) = &self.retention_cleanup_handle {
            handle.abort();
        }
        for handle in self.task_handles.values() {
            handle.abort();
        }
    }
}
