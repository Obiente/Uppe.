// Removed unstable feature to avoid warnings
use std::{path};

use clap::{Parser, Subcommand, crate_authors, crate_version};

mod config;
mod models;
mod pool;
mod database;
mod monitoring;
mod crypto;
mod p2p;
mod orchestrator;
mod tui;
mod validation;
mod location;

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
    let cfg = config::Config::from_config(cli.config.as_ref())
        .expect("Failed to load configuration");

    // Initialize database pool - use shared database location
    // Default to shared/data/libsql.db in project root (2 levels up from apps/service)
    let db_path = std::env::var("DATABASE_LIBSQL_PATH").unwrap_or_else(|_| {
        // Try to find project root by looking for Cargo.toml or turbo.json
        let default_path = if std::path::Path::new("../../shared/data").exists() 
            || std::fs::create_dir_all("../../shared/data").is_ok() {
            "../../shared/data/libsql.db".to_string()
        } else if std::path::Path::new("shared/data").exists() 
            || std::fs::create_dir_all("shared/data").is_ok() {
            "shared/data/libsql.db".to_string()
        } else {
            "libsql.db".to_string()
        };
        default_path
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
        tracing::info!("Initializing dynamic IP-based location tracking (update interval: {}s, privacy: {:?})", 
                      location_update_interval, cfg.preferences.location_privacy);
        location::init_location_cache(location_update_interval, cfg.preferences.location_privacy);
        location::update_location_from_ip(); // Trigger first update
    }

    match cli.command.unwrap_or(Commands::Run) {
        Commands::Run => {
            tracing::info!("Starting Uppe. service...");
            tracing::info!("P2P network enabled: {}", cfg.preferences.use_peerup_layer);
            orchestrator::Orchestrator::new(cfg, pool).await?;
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
                            println!("- {} [{}] -> {} (every {}s, timeout {}s)", m.name, m.check_type, m.target, m.interval_seconds, m.timeout_seconds);
                        }
                    }
                }
                MonitorCmd::Add { name, target, check_type, interval, timeout } => {
                    let mut monitor = database::models::Monitor::new(name, target, check_type);
                    monitor.interval_seconds = interval;
                    monitor.timeout_seconds = timeout;
                    let id = dbi.save_monitor(&monitor).await?;
                    println!("Added monitor with id {} and uuid {}", id, monitor.uuid);
                }
            }
        }
        Commands::Tui => {
            // Get peer ID and P2P status
            let keypair_path = path::PathBuf::from("uppe_keypair.key");
            let peer_id = if let Ok(kp) = crypto::load_or_generate_keypair(&keypair_path) {
                kp.public_key_hex()
            } else {
                "unknown".to_string()
            };
            let p2p_enabled = cfg.preferences.use_peerup_layer;
            tui::run_tui_with_p2p(pool, peer_id, p2p_enabled).await?;
        }
    }

    Ok(())
}
