use anyhow::Result;
use libsql::Connection;

/// Schema version - increment when making schema changes
const SCHEMA_VERSION: i32 = 5;

/// Run database migrations
///
/// This is the single source of truth for database schema.
/// The Go API should NOT run migrations - it only reads data.
pub async fn run_migrations(conn: &Connection) -> Result<()> {
    // Create schema_migrations table first (tracks applied migrations)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at INTEGER NOT NULL,
            description TEXT
        )",
        (),
    )
    .await?;

    // Check current schema version
    let current_version = get_current_version(conn).await?;

    if current_version >= SCHEMA_VERSION {
        tracing::info!("Database schema is up to date (version {})", current_version);
        return Ok(());
    }

    tracing::info!("Running migrations from version {} to {}", current_version, SCHEMA_VERSION);

    // Run migrations based on current version
    if current_version < 1 {
        run_migration_v1(conn).await?;
        record_migration(conn, 1, "Initial schema").await?;
    }

    if current_version < 2 {
        run_migration_v2(conn).await?;
        record_migration(conn, 2, "Add HTTP-specific columns to monitors").await?;
    }

    if current_version < 3 {
        run_migration_v3(conn).await?;
        record_migration(conn, 3, "Add status pages, settings, and network tables").await?;
    }

    if current_version < 4 {
        run_migration_v4(conn).await?;
        record_migration(conn, 4, "Add public/private monitor visibility and orchestration fields").await?;
    }

    if current_version < 5 {
        run_migration_v5(conn).await?;
        record_migration(conn, 5, "Add retention_until and P2P sync columns to peer_results").await?;
    }

    tracing::info!(
        "Database migrations completed successfully (now at version {})",
        SCHEMA_VERSION
    );
    Ok(())
}

/// Get current schema version from database
async fn get_current_version(conn: &Connection) -> Result<i32> {
    let mut rows = conn.query("SELECT MAX(version) FROM schema_migrations", ()).await?;

    if let Some(row) = rows.next().await? {
        let version: Option<i32> = row.get(0)?;
        Ok(version.unwrap_or(0))
    } else {
        Ok(0)
    }
}

/// Record that a migration was applied
async fn record_migration(conn: &Connection, version: i32, description: &str) -> Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    conn.execute(
        "INSERT INTO schema_migrations (version, applied_at, description) VALUES (?, ?, ?)",
        libsql::params![version, now, description],
    )
    .await?;

    tracing::info!("Applied migration v{}: {}", version, description);
    Ok(())
}

/// Migration v1: Initial schema
/// Creates monitors, monitor_results, and peer_results tables
async fn run_migration_v1(conn: &Connection) -> Result<()> {
    // Create monitors table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS monitors (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uuid TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            target TEXT NOT NULL,
            check_type TEXT NOT NULL,
            interval_seconds INTEGER NOT NULL DEFAULT 30,
            timeout_seconds INTEGER NOT NULL DEFAULT 10,
            enabled INTEGER NOT NULL DEFAULT 1,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        (),
    )
    .await?;

    // Create monitor_results table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS monitor_results (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            monitor_uuid TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            status TEXT NOT NULL,
            latency_ms INTEGER,
            status_code INTEGER,
            error_message TEXT,
            peer_id TEXT NOT NULL,
            signature BLOB,
            created_at INTEGER NOT NULL,
            city TEXT,
            country TEXT,
            region TEXT,
            FOREIGN KEY (monitor_uuid) REFERENCES monitors(uuid) ON DELETE CASCADE
        )",
        (),
    )
    .await?;

    // Create peer_results table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS peer_results (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            monitor_uuid TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            status TEXT NOT NULL,
            latency_ms INTEGER,
            status_code INTEGER,
            error_message TEXT,
            peer_id TEXT NOT NULL,
            signature BLOB NOT NULL,
            verified INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            city TEXT,
            country TEXT,
            region TEXT,
            source_peer_id TEXT,
            synced_from_peer INTEGER DEFAULT 0,
            retention_until INTEGER
        )",
        (),
    )
    .await?;

    // Create indexes
    conn.execute("CREATE INDEX IF NOT EXISTS idx_monitors_uuid ON monitors(uuid)", ())
        .await?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_monitors_enabled ON monitors(enabled)", ())
        .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_monitors_created_at ON monitors(created_at DESC)",
        (),
    )
    .await?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_monitor_results_monitor_uuid ON \
         monitor_results(monitor_uuid)",
        (),
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_monitor_results_timestamp ON monitor_results(timestamp \
         DESC)",
        (),
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_monitor_results_monitor_timestamp ON \
         monitor_results(monitor_uuid, timestamp DESC)",
        (),
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_monitor_results_peer_id ON monitor_results(peer_id)",
        (),
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_monitor_results_status ON monitor_results(status)",
        (),
    )
    .await?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_peer_results_monitor_uuid ON peer_results(monitor_uuid)",
        (),
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_peer_results_timestamp ON peer_results(timestamp DESC)",
        (),
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_peer_results_monitor_timestamp ON \
         peer_results(monitor_uuid, timestamp DESC)",
        (),
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_peer_results_peer_id ON peer_results(peer_id)",
        (),
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_peer_results_verified ON peer_results(verified)",
        (),
    )
    .await?;

    Ok(())
}

/// Migration v2: Add HTTP-specific columns to monitors table
/// Adds expected_status_codes, headers, body, user_id for frontend/API compatibility
async fn run_migration_v2(conn: &Connection) -> Result<()> {
    // Add expected_status_codes column (JSON array of status codes like ["200", "201"])
    conn.execute("ALTER TABLE monitors ADD COLUMN expected_status_codes TEXT DEFAULT '[]'", ())
        .await?;

    // Add headers column (JSON object of HTTP headers)
    conn.execute("ALTER TABLE monitors ADD COLUMN headers TEXT DEFAULT '{}'", ())
        .await?;

    // Add body column (request body for POST/PUT requests)
    conn.execute("ALTER TABLE monitors ADD COLUMN body TEXT DEFAULT ''", ()).await?;

    // Add user_id column (for multi-user support)
    conn.execute("ALTER TABLE monitors ADD COLUMN user_id TEXT", ()).await?;

    // Set default expected_status_codes for HTTP monitors
    conn.execute(
        "UPDATE monitors SET expected_status_codes = '[\"200\"]' WHERE check_type = 'Http' AND \
         expected_status_codes = '[]'",
        (),
    )
    .await?;

    tracing::info!("Added HTTP-specific columns to monitors table");
    Ok(())
}

/// Migration v3: Add status pages, settings, incidents, and network tables
async fn run_migration_v3(conn: &Connection) -> Result<()> {
    // ============================================================
    // Status Pages - Public status pages for services
    // ============================================================
    conn.execute(
        "CREATE TABLE IF NOT EXISTS status_pages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uuid TEXT NOT NULL UNIQUE,
            title TEXT NOT NULL,
            slug TEXT NOT NULL UNIQUE,
            description TEXT DEFAULT '',
            custom_domain TEXT,
            logo_url TEXT,
            primary_color TEXT DEFAULT '#3B82F6',
            is_active INTEGER NOT NULL DEFAULT 1,
            visits INTEGER NOT NULL DEFAULT 0,
            user_id TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        (),
    )
    .await?;

    // Status page to monitors mapping (many-to-many)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS status_page_monitors (
            status_page_id TEXT NOT NULL,
            monitor_uuid TEXT NOT NULL,
            display_order INTEGER DEFAULT 0,
            PRIMARY KEY (status_page_id, monitor_uuid),
            FOREIGN KEY (status_page_id) REFERENCES status_pages(uuid) ON DELETE CASCADE,
            FOREIGN KEY (monitor_uuid) REFERENCES monitors(uuid) ON DELETE CASCADE
        )",
        (),
    )
    .await?;

    // ============================================================
    // Incidents - Track service incidents and outages
    // ============================================================
    conn.execute(
        "CREATE TABLE IF NOT EXISTS incidents (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uuid TEXT NOT NULL UNIQUE,
            title TEXT NOT NULL,
            description TEXT,
            status TEXT NOT NULL DEFAULT 'investigating',
            severity TEXT NOT NULL DEFAULT 'minor',
            monitor_uuid TEXT,
            started_at INTEGER NOT NULL,
            resolved_at INTEGER,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            FOREIGN KEY (monitor_uuid) REFERENCES monitors(uuid) ON DELETE SET NULL
        )",
        (),
    )
    .await?;

    // Incident updates (timeline)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS incident_updates (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            incident_uuid TEXT NOT NULL,
            status TEXT NOT NULL,
            message TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            FOREIGN KEY (incident_uuid) REFERENCES incidents(uuid) ON DELETE CASCADE
        )",
        (),
    )
    .await?;

    // ============================================================
    // Settings - User and node configuration
    // ============================================================
    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        (),
    )
    .await?;

    // Insert default settings
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let default_settings = vec![
        ("display_name", "Uppe. Node"),
        ("email", ""),
        ("timezone", "UTC"),
        ("email_notifications", "true"),
        ("incident_reports", "false"),
        ("network_updates", "true"),
        ("discovery_method", "dht_and_bootstrap"),
        ("contribute_to_network", "true"),
        ("max_bandwidth_mb_per_day", "100"),
        ("max_concurrent_checks", "50"),
        ("auto_accept_requests", "true"),
        ("default_interval_seconds", "60"),
        ("default_timeout_seconds", "10"),
        ("detailed_logging", "false"),
        ("result_retention_days", "30"),
        ("cluster_name", "My Cluster"),
        ("cluster_is_public", "true"),
        ("cluster_max_size", "25"),
        ("cluster_join_policy", "open"),
        ("cluster_min_contribution_score", "1.5"),
    ];

    for (key, value) in default_settings {
        conn.execute(
            "INSERT OR IGNORE INTO settings (key, value, updated_at) VALUES (?, ?, ?)",
            libsql::params![key, value, now],
        )
        .await?;
    }

    // ============================================================
    // Network Stats - Track P2P network statistics
    // ============================================================
    conn.execute(
        "CREATE TABLE IF NOT EXISTS network_stats (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp INTEGER NOT NULL,
            total_peers INTEGER DEFAULT 0,
            online_peers INTEGER DEFAULT 0,
            checks_performed INTEGER DEFAULT 0,
            checks_received INTEGER DEFAULT 0,
            bandwidth_used_mb INTEGER DEFAULT 0
        )",
        (),
    )
    .await?;

    // ============================================================
    // Peers - Known peers in the network
    // ============================================================
    conn.execute(
        "CREATE TABLE IF NOT EXISTS peers (
            peer_id TEXT PRIMARY KEY,
            location_country TEXT,
            location_region TEXT,
            location_city TEXT,
            status TEXT NOT NULL DEFAULT 'online',
            checks_per_day INTEGER DEFAULT 0,
            last_seen INTEGER NOT NULL,
            uptime_percentage REAL DEFAULT 100.0,
            contribution_score REAL DEFAULT 1.0,
            joined_at INTEGER NOT NULL
        )",
        (),
    )
    .await?;

    // Create indexes
    conn.execute("CREATE INDEX IF NOT EXISTS idx_status_pages_slug ON status_pages(slug)", ())
        .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_status_pages_active ON status_pages(is_active)",
        (),
    )
    .await?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_incidents_status ON incidents(status)", ())
        .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_incidents_monitor ON incidents(monitor_uuid)",
        (),
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_incident_updates_incident ON \
         incident_updates(incident_uuid)",
        (),
    )
    .await?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_network_stats_timestamp ON network_stats(timestamp DESC)",
        (),
    )
    .await?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_peers_status ON peers(status)", ())
        .await?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_peers_last_seen ON peers(last_seen DESC)", ())
        .await?;

    tracing::info!("Added status pages, settings, incidents, and network tables");
    Ok(())
}

/// Migration v4: Add public/private monitor visibility
async fn run_migration_v4(conn: &Connection) -> Result<()> {
    tracing::info!("Running migration v4: Add monitor visibility fields");

    // Add visibility columns to monitors table
    conn.execute(
        "ALTER TABLE monitors ADD COLUMN visibility TEXT NOT NULL DEFAULT 'Private'",
        (),
    )
    .await?;

    conn.execute(
        "ALTER TABLE monitors ADD COLUMN public_domain TEXT",
        (),
    )
    .await?;

    conn.execute(
        "ALTER TABLE monitors ADD COLUMN public_display_name TEXT",
        (),
    )
    .await?;

    conn.execute(
        "ALTER TABLE monitors ADD COLUMN owner_peer_id TEXT",
        (),
    )
    .await?;

    // Create indexes for visibility queries
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_monitors_visibility ON monitors(visibility)",
        (),
    )
    .await?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_monitors_public_domain ON monitors(public_domain) \
         WHERE public_domain IS NOT NULL",
        (),
    )
    .await?;

    // Create public_monitor_groups table for orchestration consensus
    conn.execute(
        "CREATE TABLE IF NOT EXISTS public_monitor_groups (
            domain TEXT PRIMARY KEY,
            display_name TEXT NOT NULL,
            participating_peers TEXT NOT NULL,
            schedule_json TEXT NOT NULL,
            total_checks INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            last_updated INTEGER NOT NULL
        )",
        (),
    )
    .await?;

    // Create orchestration_votes table for consensus
    conn.execute(
        "CREATE TABLE IF NOT EXISTS orchestration_votes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            domain TEXT NOT NULL,
            voter_peer_id TEXT NOT NULL,
            schedule_json TEXT NOT NULL,
            signature TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            UNIQUE(domain, voter_peer_id)
        )",
        (),
    )
    .await?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_orchestration_votes_domain ON orchestration_votes(domain)",
        (),
    )
    .await?;

    tracing::info!("Migration v4 completed successfully");

    Ok(())
}

/// Migration v5: Add retention_until and P2P sync columns to peer_results
/// For databases upgraded from earlier versions that may be missing these columns
async fn run_migration_v5(conn: &Connection) -> Result<()> {
    tracing::info!("Running migration v5: Add retention_until and P2P sync columns to peer_results");

    // Add source_peer_id column if it doesn't exist
    match conn.execute(
        "ALTER TABLE peer_results ADD COLUMN source_peer_id TEXT",
        (),
    )
    .await
    {
        Ok(_) => tracing::debug!("Added source_peer_id column"),
        Err(e) if e.to_string().contains("duplicate column") => {
            tracing::debug!("source_peer_id column already exists")
        }
        Err(e) => return Err(e.into()),
    }

    // Add synced_from_peer column if it doesn't exist
    match conn.execute(
        "ALTER TABLE peer_results ADD COLUMN synced_from_peer INTEGER DEFAULT 0",
        (),
    )
    .await
    {
        Ok(_) => tracing::debug!("Added synced_from_peer column"),
        Err(e) if e.to_string().contains("duplicate column") => {
            tracing::debug!("synced_from_peer column already exists")
        }
        Err(e) => return Err(e.into()),
    }

    // Add retention_until column if it doesn't exist
    match conn.execute(
        "ALTER TABLE peer_results ADD COLUMN retention_until INTEGER",
        (),
    )
    .await
    {
        Ok(_) => tracing::debug!("Added retention_until column"),
        Err(e) if e.to_string().contains("duplicate column") => {
            tracing::debug!("retention_until column already exists")
        }
        Err(e) => return Err(e.into()),
    }

    tracing::info!("Migration v5 completed successfully");
    Ok(())
}
