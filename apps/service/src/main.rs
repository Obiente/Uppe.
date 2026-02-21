// Removed unstable feature to avoid warnings
use std::path;

use clap::{Parser, Subcommand, crate_authors, crate_version};

mod config;
mod crypto;
mod database;
mod location;
mod models;
mod monitoring;
mod orchestrator;
mod p2p;
mod pool;
mod tui;
mod validation;

#[derive(Subcommand, Debug)]
enum MonitorCmd {
    /// List all monitors
    List,
    /// Add a new monitor
    Add {
        /// Name of the monitor
        #[arg(long)]
        name: String,
        /// Target (URL/host)
        #[arg(long)]
        target: String,
        /// Check type (http, https, tcp, icmp)
        #[arg(long, default_value = "http")]
        check_type: String,
        /// Interval in seconds
        #[arg(long, default_value_t = 30)]
        interval: u64,
        /// Timeout in seconds
        #[arg(long, default_value_t = 10)]
        timeout: u64,
    },
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run the Uppe. service (orchestrator)
    Run,
    /// Run database migrations
    Migrate,
    /// Monitor management commands
    Monitor {
        #[command(subcommand)]
        cmd: MonitorCmd,
    },
    /// Launch interactive TUI
    Tui,
}

#[derive(Parser, Debug)]
#[command(about = "Uppe. - Monitoring that doesn't let you down!", long_about = None)]
struct Args {
    #[arg(short = 'V', long)]
    /// Print version
    version: bool,

    #[arg(long)]
    /// Path to specific config file
    config: Option<path::PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_level(true)
        .init();

    let cli = Args::parse();

    if cli.version {
        let authors = crate_authors!().split(':').collect::<Vec<&str>>().join("\", \"");
        println!("Uppe. service {} - Authors: \"{}\"", crate_version!(), authors);
        return Ok(());
    }

    // Load configuration
    let cfg =
        config::Config::from_config(cli.config.as_ref()).expect("Failed to load configuration");

    // Initialize database pool - use shared database location
    // Default to shared/data/libsql.db in project root, using CARGO_MANIFEST_DIR when available
    let db_path = std::env::var("DATABASE_LIBSQL_PATH").unwrap_or_else(|_| {
        use std::path::PathBuf;

        // Build candidate paths in priority order
        let mut candidates: Vec<PathBuf> = Vec::new();

        // Prefer a path relative to the workspace root (two levels up from this crate),
        // using CARGO_MANIFEST_DIR as a stable base instead of the current working directory
        if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
            let mut workspace_root = PathBuf::from(manifest_dir);
            workspace_root.pop(); // up from apps/service to apps
            workspace_root.pop(); // up from apps to workspace root
            candidates.push(workspace_root.join("shared").join("data").join("libsql.db"));
        }

        // Fallbacks relative to the current working directory, kept for compatibility
        candidates.push(PathBuf::from("../../shared/data/libsql.db"));
        candidates.push(PathBuf::from("shared/data/libsql.db"));
        candidates.push(PathBuf::from("libsql.db"));

        // Only select a path whose parent directory either exists or can be created successfully
        for candidate in candidates {
            if let Some(parent) = candidate.parent() {
                if parent.exists() || std::fs::create_dir_all(parent).is_ok() {
                    return candidate.to_string_lossy().into_owned();
                }
            } else {
                // No parent directory (e.g., "libsql.db" in current dir) — accept it directly
                return candidate.to_string_lossy().into_owned();
            }
        }

        // As a last resort, fall back to a database file in the current directory
        "libsql.db".to_string()
    });

    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let db = libsql::Builder::new_local(&db_path).build().await?;

    let manager = pool::LibsqlManager::new(db);
    let pool: pool::LibsqlPool = deadpool::managed::Pool::builder(manager)
        .config(deadpool::managed::PoolConfig::default())
        .build()
        .expect("Failed to build database pool");

    // Initialize location system
    let location_update_interval = cfg.preferences.location_update_interval_secs;

    // Check if user has manually set location via environment variables
    let manual_location = location::Location::new(
        std::env::var("UPPE_LOCATION_CITY").ok(),
        std::env::var("UPPE_LOCATION_COUNTRY").ok(),
        std::env::var("UPPE_LOCATION_REGION").ok(),
    );

    let has_manual_location = manual_location.city.is_some()
        || manual_location.country.is_some()
        || manual_location.region.is_some();

    if has_manual_location {
        // User manually configured location - use it without auto-updates
        tracing::info!("Using manually configured location: {}", manual_location.display());
        location::init_location(manual_location);
    } else if cfg.preferences.location_privacy == config::LocationPrivacy::Disabled {
        // Location tracking disabled
        tracing::info!("Location tracking is disabled");
        location::init_location(location::Location::unknown());
    } else {
        // Auto-detect location from IP with privacy settings
        tracing::info!(
            "Initializing dynamic IP-based location tracking (update interval: {}s, privacy: {:?})",
            location_update_interval,
            cfg.preferences.location_privacy
        );
        location::init_location_cache(location_update_interval, cfg.preferences.location_privacy);
        location::update_location_from_ip(); // Trigger first update
    }

    match cli.command.unwrap_or(Commands::Run) {
        Commands::Run => {
            tracing::info!("Starting Uppe. service...");
            tracing::info!("P2P network enabled: {}", cfg.preferences.use_peerup_layer);

            // Use LocalSet for P2P network (libp2p Swarm is !Send)
            let local = tokio::task::LocalSet::new();
            local
                .run_until(async move { orchestrator::Orchestrator::start(cfg, pool).await })
                .await?;
        }
        Commands::Migrate => {
            tracing::info!("Running database migrations...");
            let conn = pool.get().await?;
            database::initialize_database(&conn).await?;
            println!("Migrations completed.");
        }
        Commands::Monitor { cmd } => {
            use database::{Database, DatabaseImpl};
            let dbi = DatabaseImpl::new_from_pool(pool);
            match cmd {
                MonitorCmd::List => {
                    let monitors = dbi.get_enabled_monitors().await?;
                    if monitors.is_empty() {
                        println!("No monitors found.");
                    } else {
                        for m in monitors {
                            println!(
                                "- {} [{}] -> {} (every {}s, timeout {}s)",
                                m.name,
                                m.check_type,
                                m.target,
                                m.interval_seconds,
                                m.timeout_seconds
                            );
                        }
                    }
                }
                MonitorCmd::Add { name, target, check_type, interval, timeout } => {
                    // Validate inputs before creating monitor
                    use crate::validation::*;

                    let name_result = validate_monitor_name(&name);
                    if !name_result.is_valid {
                        eprintln!("Error: {}", name_result.error.unwrap_or_default());
                        std::process::exit(1);
                    }

                    let target_result = validate_monitor_target(&target, &check_type);
                    if !target_result.is_valid {
                        eprintln!("Error: {}", target_result.error.unwrap_or_default());
                        std::process::exit(1);
                    }

                    let interval_result = validate_interval(interval);
                    if !interval_result.is_valid {
                        eprintln!("Error: {}", interval_result.error.unwrap_or_default());
                        std::process::exit(1);
                    }

                    let timeout_result = validate_timeout(timeout, interval);
                    if !timeout_result.is_valid {
                        eprintln!("Error: {}", timeout_result.error.unwrap_or_default());
                        std::process::exit(1);
                    }

                    let mut monitor = database::models::Monitor::new(name, target, check_type);
                    monitor.interval_seconds = interval;
                    monitor.timeout_seconds = timeout;
                    let id = dbi.save_monitor(&monitor).await?;
                    println!("Added monitor with id {} and uuid {}", id, monitor.uuid);
                }
            }
        }
        Commands::Tui => {
            // Load keypair for peer ID
            let keypair_path = std::env::var("UPPE_KEYPAIR_PATH")
                .unwrap_or_else(|_| "uppe_keypair.key".to_string());
            let keypair_path = path::PathBuf::from(keypair_path);
            let peer_id = if let Ok(kp) = crypto::load_or_generate_keypair(&keypair_path) {
                kp.public_key_hex()
            } else {
                "unknown".to_string()
            };
            let p2p_enabled = cfg.preferences.use_peerup_layer;

            // TUI is a read-only viewer — the backend must be running separately
            // via `uppe-service run` for live data to flow.
            let local = tokio::task::LocalSet::new();
            local
                .run_until(async move { tui::run_tui_with_p2p(pool, peer_id, p2p_enabled).await })
                .await?;
        }
    }

    Ok(())
}
