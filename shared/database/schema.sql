-- ============================================================================
-- Uppe. Database Schema (Source of Truth)
-- ============================================================================
-- 
-- This file documents the canonical database schema.
-- The Rust service (apps/service) is responsible for running migrations.
-- The Go API (apps/server) reads from this schema but does NOT run migrations.
--
-- Schema Version: 2.0.0
-- Last Updated: 2026-01-11
-- ============================================================================

-- ============================================================================
-- Table: monitors
-- ============================================================================
-- Stores monitor configurations created by users.
-- Primary key is an auto-increment integer.
-- UUID is used for external references (API, P2P network).
--
-- Managed by: Rust Service
-- Read by: Go API, Frontend
-- ============================================================================

CREATE TABLE IF NOT EXISTS monitors (
    -- Primary key (internal reference)
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    
    -- UUID for external reference (API, P2P)
    uuid TEXT NOT NULL UNIQUE,
    
    -- Monitor configuration
    name TEXT NOT NULL,                          -- Display name
    target TEXT NOT NULL,                        -- URL or host to monitor
    check_type TEXT NOT NULL,                    -- 'Http', 'Tcp', 'Icmp'
    interval_seconds INTEGER NOT NULL DEFAULT 30, -- Check frequency
    timeout_seconds INTEGER NOT NULL DEFAULT 10,  -- Max wait time
    
    -- HTTP-specific fields (added in v2)
    expected_status_codes TEXT DEFAULT '[]',     -- JSON array: ["200", "201"]
    headers TEXT DEFAULT '{}',                   -- JSON object: {"User-Agent": "..."}
    body TEXT DEFAULT '',                        -- Request body for POST/PUT
    
    -- Status & ownership
    enabled INTEGER NOT NULL DEFAULT 1,          -- 0=disabled, 1=enabled
    user_id TEXT,                                -- For multi-user support
    
    -- Timestamps (Unix timestamps in seconds)
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- Indexes for monitors
CREATE INDEX IF NOT EXISTS idx_monitors_uuid ON monitors(uuid);
CREATE INDEX IF NOT EXISTS idx_monitors_enabled ON monitors(enabled);
CREATE INDEX IF NOT EXISTS idx_monitors_created_at ON monitors(created_at DESC);

-- ============================================================================
-- Table: monitor_results  
-- ============================================================================
-- Stores results of monitoring checks performed by this node.
-- Each row represents one check execution.
--
-- Managed by: Rust Service (inserts)
-- Read by: Go API (queries for dashboard/analytics)
-- ============================================================================

CREATE TABLE IF NOT EXISTS monitor_results (
    -- Primary key
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    
    -- Foreign key to monitors table (using UUID for flexibility)
    monitor_uuid TEXT NOT NULL,
    
    -- Result data
    timestamp INTEGER NOT NULL,                  -- When check was performed (Unix)
    status TEXT NOT NULL,                        -- 'Up', 'Down', 'Timeout', 'Error', 'Degraded'
    latency_ms INTEGER,                          -- Response time in milliseconds
    status_code INTEGER,                         -- HTTP status code (if applicable)
    error_message TEXT,                          -- Error details if failed
    
    -- Peer identification and verification
    peer_id TEXT NOT NULL,                       -- Public key of node that ran check
    signature BLOB,                              -- Cryptographic signature of result
    
    -- Metadata
    created_at INTEGER NOT NULL,                 -- When row was inserted (Unix)
    
    -- Location (GeoIP of monitoring node)
    city TEXT,
    country TEXT,
    region TEXT,
    
    -- Foreign key constraint
    FOREIGN KEY (monitor_uuid) REFERENCES monitors(uuid) ON DELETE CASCADE
);

-- Indexes for monitor_results
CREATE INDEX IF NOT EXISTS idx_monitor_results_monitor_uuid ON monitor_results(monitor_uuid);
CREATE INDEX IF NOT EXISTS idx_monitor_results_timestamp ON monitor_results(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_monitor_results_monitor_timestamp ON monitor_results(monitor_uuid, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_monitor_results_peer_id ON monitor_results(peer_id);
CREATE INDEX IF NOT EXISTS idx_monitor_results_status ON monitor_results(status);

-- ============================================================================
-- Table: peer_results
-- ============================================================================
-- Stores results received from other nodes in the P2P network.
-- Separate from monitor_results to track provenance and verification.
--
-- Managed by: Rust Service (inserts from P2P network)
-- Read by: Go API (for global ping aggregation)
-- ============================================================================

CREATE TABLE IF NOT EXISTS peer_results (
    -- Primary key
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    
    -- Reference to which monitor this result is for
    monitor_uuid TEXT NOT NULL,
    
    -- Result data (same structure as monitor_results)
    timestamp INTEGER NOT NULL,
    status TEXT NOT NULL,
    latency_ms INTEGER,
    status_code INTEGER,
    error_message TEXT,
    
    -- Peer identification
    peer_id TEXT NOT NULL,                       -- Public key of remote peer
    signature BLOB NOT NULL,                     -- Required for peer results
    
    -- Verification status
    verified INTEGER NOT NULL DEFAULT 0,         -- 0=unverified, 1=verified
    
    -- Metadata
    created_at INTEGER NOT NULL,
    
    -- Location of remote peer
    city TEXT,
    country TEXT,
    region TEXT
);

-- Indexes for peer_results
CREATE INDEX IF NOT EXISTS idx_peer_results_monitor_uuid ON peer_results(monitor_uuid);
CREATE INDEX IF NOT EXISTS idx_peer_results_timestamp ON peer_results(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_peer_results_monitor_timestamp ON peer_results(monitor_uuid, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_peer_results_peer_id ON peer_results(peer_id);
CREATE INDEX IF NOT EXISTS idx_peer_results_verified ON peer_results(verified);

-- ============================================================================
-- Table: schema_migrations
-- ============================================================================
-- Tracks which migrations have been applied.
-- Used by Rust service to manage schema versions.
--
-- Managed by: Rust Service ONLY
-- ============================================================================

CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at INTEGER NOT NULL,
    description TEXT
);

-- ============================================================================
-- Status Values Reference
-- ============================================================================
-- 
-- The 'status' column in monitor_results and peer_results uses these values:
-- 
-- | Value      | Description                                    |
-- |------------|------------------------------------------------|
-- | 'Up'       | Service responded successfully                 |
-- | 'Down'     | Service did not respond                        |
-- | 'Timeout'  | Response exceeded timeout_seconds              |
-- | 'Error'    | Network error, DNS failure, etc.               |
-- | 'Degraded' | Responded but latency > degraded_threshold     |
--
-- Note: Status is stored as TEXT for readability and flexibility.
-- Go API should map these to protobuf enum values.
-- ============================================================================

-- ============================================================================
-- Column Mapping Reference (Rust <-> Go)
-- ============================================================================
--
-- monitors table:
-- | SQLite Column    | Rust Field          | Go Field (GORM)     |
-- |------------------|---------------------|---------------------|
-- | id               | id: Option<i64>     | ID int64            |
-- | uuid             | uuid: Uuid          | UUID string         |
-- | name             | name: String        | Name string         |
-- | target           | target: String      | Target string       |
-- | check_type       | check_type: String  | CheckType string    |
-- | interval_seconds | interval_seconds    | IntervalSeconds     |
-- | timeout_seconds  | timeout_seconds     | TimeoutSeconds      |
-- | enabled          | enabled: bool       | Enabled bool        |
-- | created_at       | created_at: i64     | CreatedAt int64     |
-- | updated_at       | updated_at: i64     | UpdatedAt int64     |
--
-- monitor_results table:
-- | SQLite Column    | Rust Field          | Go Field (GORM)     |
-- |------------------|---------------------|---------------------|
-- | id               | id: Option<i64>     | ID int64            |
-- | monitor_uuid     | monitor_uuid: Uuid  | MonitorUUID string  |
-- | timestamp        | timestamp: i64      | Timestamp int64     |
-- | status           | status: String      | Status string       |
-- | latency_ms       | latency_ms: Option  | LatencyMs *int64    |
-- | status_code      | status_code: Option | StatusCode *int32   |
-- | error_message    | error_message: Opt  | ErrorMessage *str   |
-- | peer_id          | peer_id: String     | PeerID string       |
-- | signature        | signature: Vec<u8>  | Signature []byte    |
-- | created_at       | created_at: i64     | CreatedAt int64     |
-- | city             | city: Option<Str>   | City *string        |
-- | country          | country: Option     | Country *string     |
-- | region           | region: Option      | Region *string      |
--
-- ============================================================================
