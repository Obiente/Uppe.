package libsql

import (
	"context"
	"crypto/rand"
	"database/sql"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"net/url"
	"os"
	"path/filepath"
	"strings"
	"time"

	commonv1 "github.com/Obiente/Uppe/apps/server/gen/common/v1"
	monitorv1 "github.com/Obiente/Uppe/apps/server/gen/monitor/v1"
	resultv1 "github.com/Obiente/Uppe/apps/server/gen/result/v1"
	"github.com/Obiente/Uppe/apps/server/internal/db/types"
	"github.com/Obiente/Uppe/apps/server/internal/models"
	_ "modernc.org/sqlite"
)

// Minimum required schema version (must match Rust service migrations)
// v4 added visibility, owner_peer_id, public_domain, public_display_name to monitors.
const MinRequiredSchemaVersion = 8

func min(a, b int) int {
	if a < b {
		return a
	}
	return b
}

// generateUUID generates a simple UUID v4
func generateUUID() string {
	b := make([]byte, 16)
	if _, err := rand.Read(b); err != nil {
		panic("operating system random source unavailable")
	}
	b[6] = (b[6] & 0x0f) | 0x40 // Version 4
	b[8] = (b[8] & 0x3f) | 0x80 // Variant
	return hex.EncodeToString(b[:4]) + "-" +
		hex.EncodeToString(b[4:6]) + "-" +
		hex.EncodeToString(b[6:8]) + "-" +
		hex.EncodeToString(b[8:10]) + "-" +
		hex.EncodeToString(b[10:])
}

// boolToInt converts bool to int for SQLite
func boolToInt(b bool) int {
	if b {
		return 1
	}
	return 0
}

// Helper function to convert ResultStatus enum to string
func resultStatusToString(status resultv1.ResultStatus) string {
	switch status {
	case resultv1.ResultStatus_RESULT_STATUS_UP:
		return "Up"
	case resultv1.ResultStatus_RESULT_STATUS_DOWN:
		return "Down"
	case resultv1.ResultStatus_RESULT_STATUS_TIMEOUT:
		return "Timeout"
	case resultv1.ResultStatus_RESULT_STATUS_ERROR:
		return "Error"
	default:
		return "Error"
	}
}

// Unknown or future statuses are missing monitoring evidence, never a target failure.
func parseResultStatus(status string) resultv1.ResultStatus {
	switch strings.ToLower(status) {
	case "up":
		return resultv1.ResultStatus_RESULT_STATUS_UP
	case "down":
		return resultv1.ResultStatus_RESULT_STATUS_DOWN
	case "degraded":
		return resultv1.ResultStatus_RESULT_STATUS_DEGRADED
	case "timeout":
		return resultv1.ResultStatus_RESULT_STATUS_TIMEOUT
	case "error":
		return resultv1.ResultStatus_RESULT_STATUS_ERROR
	default:
		return resultv1.ResultStatus_RESULT_STATUS_UNSPECIFIED
	}
}

// LibSQLDatabase implements the Database interface using LibSQL (SQLite-compatible)
//
// IMPORTANT: This API server does NOT manage database migrations.
// The Rust service (apps/service) is the single source of truth for schema.
// This API only reads data from the shared database.
type LibSQLDatabase struct {
	db *sql.DB
}

// NewWithRetry creates a new LibSQL database connection, retrying until schema is ready
// This allows the Go API to start concurrently with the Rust service
func NewWithRetry(databaseURL string, maxRetries int, retryInterval time.Duration) (*LibSQLDatabase, error) {
	var lastErr error

	for i := 0; i < maxRetries; i++ {
		db, err := New(databaseURL)
		if err == nil {
			return db, nil
		}

		lastErr = err

		// Log retry attempt (only after first failure)
		if i == 0 {
			fmt.Printf("Waiting for database schema (Rust service must initialize first)...\n")
		}

		time.Sleep(retryInterval)
	}

	return nil, fmt.Errorf("database not ready after %d attempts: %w", maxRetries, lastErr)
}

// New creates a new LibSQL database connection
// NOTE: Does NOT run migrations - schema is managed by Rust service
func New(databaseURL string) (*LibSQLDatabase, error) {
	// For local SQLite files, use sqlite3 driver
	// Remove file:// prefix if present for sqlite3 driver
	dbPath := databaseURL
	if len(dbPath) > 7 && dbPath[:7] == "file://" {
		dbPath = dbPath[7:]
	}

	if _, err := os.Stat(dbPath); err != nil {
		return nil, fmt.Errorf("database file is not ready: %w", err)
	}
	absolute, err := filepath.Abs(dbPath)
	if err != nil {
		return nil, err
	}
	uriPath := filepath.ToSlash(absolute)
	if filepath.VolumeName(absolute) != "" {
		uriPath = "/" + uriPath
	}
	source := url.URL{Scheme: "file", Path: uriPath}
	params := url.Values{}
	params.Add("_pragma", "busy_timeout(5000)")
	params.Add("_pragma", "foreign_keys(1)")
	params.Add("_pragma", "journal_mode(WAL)")
	source.RawQuery = params.Encode()
	sqlDB, err := sql.Open("sqlite", source.String())
	if err != nil {
		return nil, fmt.Errorf("failed to open database: %w", err)
	}

	sqlDB.SetMaxOpenConns(4)
	sqlDB.SetMaxIdleConns(4)
	// Test connection
	if err := sqlDB.Ping(); err != nil {
		sqlDB.Close()
		return nil, fmt.Errorf("failed to ping database: %w", err)
	}

	db := &LibSQLDatabase{
		db: sqlDB,
	}

	// Verify schema compatibility (do NOT run migrations)
	if err := db.verifySchema(context.Background()); err != nil {
		sqlDB.Close()
		return nil, fmt.Errorf("schema verification failed: %w", err)
	}

	return db, nil
}

// verifySchema checks that the database schema is compatible
// Returns error if schema is missing or incompatible
func (d *LibSQLDatabase) verifySchema(ctx context.Context) error {
	// Check if schema_migrations table exists
	var count int
	err := d.db.QueryRowContext(ctx,
		"SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='schema_migrations'",
	).Scan(&count)
	if err != nil {
		return fmt.Errorf("failed to check schema_migrations table: %w", err)
	}

	if count == 0 {
		return fmt.Errorf("database not initialized - run Rust service first to create schema")
	}

	// Check schema version
	var version int
	err = d.db.QueryRowContext(ctx, "SELECT MAX(version) FROM schema_migrations").Scan(&version)
	if err != nil {
		return fmt.Errorf("failed to get schema version: %w", err)
	}

	if version != MinRequiredSchemaVersion {
		return fmt.Errorf("schema version %d is incompatible with required version %d; use matching Rust and Go builds",
			version, MinRequiredSchemaVersion)
	}

	// Verify required tables exist
	requiredTables := []string{"monitors", "monitor_results", "peer_results", "settings", "status_pages", "status_page_monitors", "peers", "audit_events"}
	for _, table := range requiredTables {
		var tableCount int
		err := d.db.QueryRowContext(ctx,
			"SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
			table,
		).Scan(&tableCount)
		if err != nil || tableCount == 0 {
			return fmt.Errorf("required table '%s' not found - run Rust service first", table)
		}
	}

	return nil
}

// Ping checks database connectivity
func (d *LibSQLDatabase) Ping(ctx context.Context) error {
	return d.db.PingContext(ctx)
}

// Close closes the database connection
func (d *LibSQLDatabase) Close() error {
	return d.db.Close()
}

// CreateMonitor creates a new monitor using raw SQL to match Rust schema
func (d *LibSQLDatabase) CreateMonitor(ctx context.Context, monitor *models.Monitor) error {
	// Generate UUID if not set
	if monitor.ID == "" {
		monitor.ID = generateUUID()
	}

	// Serialize JSON fields
	expectedCodesJSON, _ := json.Marshal(monitor.ExpectedStatusCodes)
	if len(monitor.ExpectedStatusCodes) == 0 {
		expectedCodesJSON = []byte(`[]`)
	}
	headersJSON, _ := json.Marshal(monitor.Headers)
	if monitor.Headers == nil {
		headersJSON = []byte(`{}`)
	}

	// Convert type to Rust format
	checkType := "http"
	switch monitor.Type {
	case monitorv1.MonitorType_MONITOR_TYPE_HTTP:
		checkType = "http"
	case monitorv1.MonitorType_MONITOR_TYPE_TCP:
		checkType = "tcp"
	case monitorv1.MonitorType_MONITOR_TYPE_ICMP:
		checkType = "icmp"
	}

	now := time.Now().Unix()

	// Determine visibility string; default to Private
	visibility := string(monitor.Visibility)
	if visibility == "" {
		visibility = "Private"
	}

	_, err := d.db.ExecContext(ctx, `
		INSERT INTO monitors (uuid, name, target, check_type, interval_seconds, timeout_seconds,
			expected_status_codes, headers, body, enabled, user_id, created_at, updated_at,
			visibility, owner_peer_id, public_domain, public_display_name)
		VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
	`,
		monitor.ID,
		monitor.Name,
		monitor.URL,
		checkType,
		monitor.IntervalSeconds,
		monitor.TimeoutSeconds,
		string(expectedCodesJSON),
		string(headersJSON),
		monitor.Body,
		boolToInt(monitor.Enabled),
		monitor.UserID,
		now,
		now,
		visibility,
		monitor.OwnerPeerID,
		monitor.PublicDomain,
		monitor.PublicDisplayName,
	)

	return err
}

// GetMonitor retrieves a monitor by UUID
func (d *LibSQLDatabase) GetMonitor(ctx context.Context, id string) (*models.Monitor, error) {
	query := `
		SELECT uuid, target, name, check_type, interval_seconds, timeout_seconds,
			COALESCE(expected_status_codes, '[]'), COALESCE(headers, '{}'),
			COALESCE(body, ''), enabled, user_id, created_at, updated_at,
			COALESCE(visibility, 'Private'), owner_peer_id, public_domain, public_display_name
		FROM monitors WHERE uuid = ?
	`

	var monitor models.Monitor
	var expectedCodesJSON, headersJSON, typeStr, visibilityStr string
	var enabled int
	var userID, ownerPeerID, publicDomain, publicDisplayName sql.NullString
	var createdAt, updatedAt int64

	err := d.db.QueryRowContext(ctx, query, id).Scan(
		&monitor.ID,
		&monitor.URL,
		&monitor.Name,
		&typeStr,
		&monitor.IntervalSeconds,
		&monitor.TimeoutSeconds,
		&expectedCodesJSON,
		&headersJSON,
		&monitor.Body,
		&enabled,
		&userID,
		&createdAt,
		&updatedAt,
		&visibilityStr,
		&ownerPeerID,
		&publicDomain,
		&publicDisplayName,
	)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil, fmt.Errorf("monitor not found: %w", sql.ErrNoRows)
		}
		return nil, fmt.Errorf("failed to get monitor: %w", err)
	}

	// Parse type
	switch typeStr {
	case "http", "Http", "HTTP", "https", "Https", "HTTPS":
		monitor.Type = monitorv1.MonitorType_MONITOR_TYPE_HTTP
	case "tcp", "Tcp", "TCP":
		monitor.Type = monitorv1.MonitorType_MONITOR_TYPE_TCP
	case "icmp", "Icmp", "ICMP":
		monitor.Type = monitorv1.MonitorType_MONITOR_TYPE_ICMP
	default:
		monitor.Type = monitorv1.MonitorType_MONITOR_TYPE_HTTP
	}

	monitor.Visibility = models.MonitorVisibility(visibilityStr)
	if monitor.Visibility == "" {
		monitor.Visibility = models.MonitorVisibilityPrivate
	}
	if ownerPeerID.Valid {
		monitor.OwnerPeerID = &ownerPeerID.String
	}
	if publicDomain.Valid {
		monitor.PublicDomain = &publicDomain.String
	}
	if publicDisplayName.Valid {
		monitor.PublicDisplayName = &publicDisplayName.String
	}

	monitor.Enabled = enabled == 1
	if userID.Valid {
		monitor.UserID = &userID.String
	}

	monitor.CreatedAt = time.Unix(createdAt, 0)
	monitor.UpdatedAt = time.Unix(updatedAt, 0)

	json.Unmarshal([]byte(expectedCodesJSON), &monitor.ExpectedStatusCodes)
	json.Unmarshal([]byte(headersJSON), &monitor.Headers)

	return &monitor, nil
}

// ListMonitors lists monitors with filters
func (d *LibSQLDatabase) ListMonitors(ctx context.Context, filters *types.MonitorFilters) ([]*models.Monitor, int, error) {
	// Build WHERE clause
	where := []string{}
	args := []interface{}{}

	if filters.UserID != nil {
		where = append(where, "user_id = ?")
		args = append(args, *filters.UserID)
	}
	if filters.Enabled != nil {
		where = append(where, "enabled = ?")
		enabledVal := 0
		if *filters.Enabled {
			enabledVal = 1
		}
		args = append(args, enabledVal)
	}
	if filters.Visibility != nil && *filters.Visibility != "" {
		where = append(where, "visibility = ?")
		args = append(args, *filters.Visibility)
	}

	whereClause := ""
	if len(where) > 0 {
		whereClause = "WHERE " + where[0]
		for i := 1; i < len(where); i++ {
			whereClause += " AND " + where[i]
		}
	}

	// Get total count
	countQuery := fmt.Sprintf("SELECT COUNT(*) FROM monitors %s", whereClause)
	var total int
	err := d.db.QueryRowContext(ctx, countQuery, args...).Scan(&total)
	if err != nil {
		return nil, 0, fmt.Errorf("failed to count monitors: %w", err)
	}

	// Get monitors - include visibility columns (schema v4+)
	query := fmt.Sprintf(`
		SELECT uuid, target, name, check_type, interval_seconds, timeout_seconds,
			COALESCE(expected_status_codes, '[]'), COALESCE(headers, '{}'),
			COALESCE(body, ''), enabled, user_id, created_at, updated_at,
			COALESCE(visibility, 'Private'), owner_peer_id, public_domain, public_display_name
		FROM monitors %s
		ORDER BY created_at DESC
		LIMIT ? OFFSET ?
	`, whereClause)

	limit := filters.PageSize
	if limit <= 0 {
		limit = 50 // default
	}
	offset := (filters.Page - 1) * limit
	if offset < 0 {
		offset = 0
	}

	args = append(args, limit, offset)
	rows, err := d.db.QueryContext(ctx, query, args...)
	if err != nil {
		return nil, 0, fmt.Errorf("failed to query monitors: %w", err)
	}
	defer rows.Close()

	monitors := []*models.Monitor{}
	for rows.Next() {
		var monitor models.Monitor
		var expectedCodesJSON, headersJSON, typeStr, visibilityStr string
		var enabled int
		var userID, ownerPeerID, publicDomain, publicDisplayName sql.NullString
		var createdAt, updatedAt int64

		err := rows.Scan(
			&monitor.ID,
			&monitor.URL,
			&monitor.Name,
			&typeStr,
			&monitor.IntervalSeconds,
			&monitor.TimeoutSeconds,
			&expectedCodesJSON,
			&headersJSON,
			&monitor.Body,
			&enabled,
			&userID,
			&createdAt,
			&updatedAt,
			&visibilityStr,
			&ownerPeerID,
			&publicDomain,
			&publicDisplayName,
		)
		if err != nil {
			return nil, 0, fmt.Errorf("failed to scan monitor: %w", err)
		}

		// Parse check_type — Rust stores as Http, Tcp, Icmp
		switch typeStr {
		case "http", "Http", "HTTP", "https", "Https", "HTTPS":
			monitor.Type = monitorv1.MonitorType_MONITOR_TYPE_HTTP
		case "tcp", "Tcp", "TCP":
			monitor.Type = monitorv1.MonitorType_MONITOR_TYPE_TCP
		case "icmp", "Icmp", "ICMP":
			monitor.Type = monitorv1.MonitorType_MONITOR_TYPE_ICMP
		default:
			monitor.Type = monitorv1.MonitorType_MONITOR_TYPE_HTTP
		}

		monitor.Visibility = models.MonitorVisibility(visibilityStr)
		if monitor.Visibility == "" {
			monitor.Visibility = models.MonitorVisibilityPrivate
		}
		if ownerPeerID.Valid {
			monitor.OwnerPeerID = &ownerPeerID.String
		}
		if publicDomain.Valid {
			monitor.PublicDomain = &publicDomain.String
		}
		if publicDisplayName.Valid {
			monitor.PublicDisplayName = &publicDisplayName.String
		}

		monitor.Enabled = enabled == 1
		if userID.Valid {
			monitor.UserID = &userID.String
		}

		monitor.CreatedAt = time.Unix(createdAt, 0)
		monitor.UpdatedAt = time.Unix(updatedAt, 0)

		json.Unmarshal([]byte(expectedCodesJSON), &monitor.ExpectedStatusCodes)
		json.Unmarshal([]byte(headersJSON), &monitor.Headers)

		monitors = append(monitors, &monitor)
	}

	return monitors, total, rows.Err()
}

// UpdateMonitor updates a monitor by UUID
func (d *LibSQLDatabase) UpdateMonitor(ctx context.Context, id string, updates *models.MonitorUpdate) error {
	// Build SET clause dynamically
	setClauses := []string{}
	args := []interface{}{}

	if updates.Name != nil {
		setClauses = append(setClauses, "name = ?")
		args = append(args, *updates.Name)
	}
	if updates.IntervalSeconds != nil {
		setClauses = append(setClauses, "interval_seconds = ?")
		args = append(args, *updates.IntervalSeconds)
	}
	if updates.TimeoutSeconds != nil {
		setClauses = append(setClauses, "timeout_seconds = ?")
		args = append(args, *updates.TimeoutSeconds)
	}
	if updates.Enabled != nil {
		setClauses = append(setClauses, "enabled = ?")
		args = append(args, boolToInt(*updates.Enabled))
	}
	if updates.Body != nil {
		setClauses = append(setClauses, "body = ?")
		args = append(args, *updates.Body)
	}
	if updates.Headers != nil {
		headersJSON, err := json.Marshal(updates.Headers)
		if err != nil {
			return fmt.Errorf("failed to marshal headers: %w", err)
		}
		setClauses = append(setClauses, "headers = ?")
		args = append(args, string(headersJSON))
	}
	if updates.PublicDomain != nil {
		setClauses = append(setClauses, "public_domain = ?")
		args = append(args, *updates.PublicDomain)
	}
	if updates.PublicDisplayName != nil {
		setClauses = append(setClauses, "public_display_name = ?")
		args = append(args, *updates.PublicDisplayName)
	}

	if updates.URL != nil {
		setClauses = append(setClauses, "target = ?")
		args = append(args, *updates.URL)
	}
	if updates.Visibility != nil {
		setClauses = append(setClauses, "visibility = ?")
		args = append(args, string(*updates.Visibility))
	}
	// Always update updated_at
	if len(setClauses) == 0 {
		return fmt.Errorf("no fields to update")
	}
	setClauses = append(setClauses, "updated_at = ?")
	args = append(args, time.Now().Unix())

	// Add the WHERE clause parameter
	args = append(args, id)

	query := fmt.Sprintf("UPDATE monitors SET %s WHERE uuid = ?",
		joinStrings(setClauses, ", "))

	_, err := d.db.ExecContext(ctx, query, args...)
	return err
}

// DeleteMonitor deletes a monitor by UUID
func (d *LibSQLDatabase) DeleteMonitor(ctx context.Context, id string) error {
	_, err := d.db.ExecContext(ctx, "DELETE FROM monitors WHERE uuid = ?", id)
	return err
}

// joinStrings joins strings with separator (helper function)
func joinStrings(strs []string, sep string) string {
	if len(strs) == 0 {
		return ""
	}
	result := strs[0]
	for i := 1; i < len(strs); i++ {
		result += sep + strs[i]
	}
	return result
}

// GetResults retrieves monitoring results
func (d *LibSQLDatabase) GetResults(ctx context.Context, query *types.ResultQuery) ([]*models.MonitoringResult, int, error) {
	// Get total count
	countQuery := `
		SELECT COUNT(*) FROM monitor_results
		WHERE monitor_uuid = ?
		AND timestamp >= ? AND timestamp <= ?
	`
	var total int
	var err error
	if !query.SkipTotal {
		err = d.db.QueryRowContext(ctx, countQuery,
			query.MonitorID,
			query.StartTime.Unix(),
			query.EndTime.Unix(),
		).Scan(&total)
		if err != nil {
			return nil, 0, fmt.Errorf("failed to count results: %w", err)
		}

	}

	// Get results
	resultsQuery := `
		SELECT id, monitor_uuid, timestamp, status, latency_ms, status_code,
			peer_id, signature, error_message, created_at,
			city, country, region
		FROM monitor_results
		WHERE monitor_uuid = ?
		AND timestamp >= ? AND timestamp <= ?
		ORDER BY timestamp DESC
		LIMIT ? OFFSET ?
	`

	limit := query.Limit
	if limit <= 0 {
		limit = 100
	}
	if limit > 1000 {
		limit = 1000
	}
	if query.Offset < 0 {
		query.Offset = 0
	}

	rows, err := d.db.QueryContext(ctx, resultsQuery,
		query.MonitorID,
		query.StartTime.Unix(),
		query.EndTime.Unix(),
		limit,
		query.Offset,
	)
	if err != nil {
		return nil, 0, fmt.Errorf("failed to query results: %w", err)
	}
	defer rows.Close()

	results := []*models.MonitoringResult{}
	for rows.Next() {
		var result models.MonitoringResult
		var rowID int64
		var statusStr string
		var latencyMs sql.NullInt64
		var statusCode sql.NullInt32
		var errorMsg sql.NullString
		var peerID sql.NullString
		var signature []byte
		var timestamp, createdAt int64
		var city, country, region sql.NullString

		err := rows.Scan(
			&rowID,
			&result.MonitorID,
			&timestamp,
			&statusStr,
			&latencyMs,
			&statusCode,
			&peerID,
			&signature,
			&errorMsg,
			&createdAt,
			&city,
			&country,
			&region,
		)
		if err != nil {
			return nil, 0, fmt.Errorf("failed to scan result: %w", err)
		}

		result.ID = fmt.Sprintf("%d", rowID)
		result.Timestamp = time.Unix(timestamp, 0)
		result.CreatedAt = time.Unix(createdAt, 0)

		if latencyMs.Valid {
			result.LatencyMs = latencyMs.Int64
		}
		if peerID.Valid {
			result.MonitorNodeID = peerID.String
		}

		// Parse status - Rust stores as Up, Down, Timeout, Error
		result.Status = parseResultStatus(statusStr)

		if statusCode.Valid {
			result.StatusCode = &statusCode.Int32
		}
		if errorMsg.Valid {
			result.ErrorMessage = &errorMsg.String
		}
		if len(signature) > 0 {
			result.Signature = signature
		}
		if city.Valid || country.Valid || region.Valid {
			result.Location = &commonv1.Location{
				City:    city.String,
				Country: country.String,
				Region:  region.String,
			}
		}

		results = append(results, &result)
	}

	return results, total, rows.Err()
}

// GetMonitorStats retrieves computed statistics for a monitor over a time period
func (d *LibSQLDatabase) GetMonitorStats(ctx context.Context, monitorID string, startTime, endTime time.Time) (*models.MonitorStats, error) {
	query := `
		SELECT
			COUNT(*) as total_checks,
			COALESCE(SUM(CASE WHEN status IN ('Up', 'UP', 'up', 'Degraded', 'degraded') THEN 1 ELSE 0 END), 0) as successful_checks,
			COALESCE(AVG(CAST(latency_ms AS REAL)), 0) as avg_latency,
			COALESCE(MIN(latency_ms), 0) as min_latency,
			COALESCE(MAX(latency_ms), 0) as max_latency
		FROM monitor_results
		WHERE monitor_uuid = ? AND timestamp >= ? AND timestamp <= ? AND lower(status) IN ('up', 'down', 'degraded', 'error', 'timeout')
	`

	stats := &models.MonitorStats{MonitorID: monitorID}
	err := d.db.QueryRowContext(ctx, query,
		monitorID,
		startTime.Unix(),
		endTime.Unix(),
	).Scan(
		&stats.TotalChecks,
		&stats.SuccessfulChecks,
		&stats.AverageLatencyMs,
		&stats.MinLatencyMs,
		&stats.MaxLatencyMs,
	)
	if err != nil {
		return nil, fmt.Errorf("failed to query monitor stats: %w", err)
	}

	stats.FailedChecks = stats.TotalChecks - stats.SuccessfulChecks
	if stats.TotalChecks > 0 {
		stats.UptimePercentage = float64(stats.SuccessfulChecks) / float64(stats.TotalChecks) * 100
	}

	return stats, nil
}

// GetAggregatedStats gets aggregated statistics bucketed by calendar period
func (d *LibSQLDatabase) GetAggregatedStats(ctx context.Context, monitorID string, startTime, endTime time.Time, period types.AggregationPeriod) ([]*models.TimeSeriesDataPoint, error) {
	// Group by calendar day using SQLite's strftime
	var bucketFmt string
	switch period {
	case types.AggregationPeriodHour:
		bucketFmt = "%Y-%m-%dT%H:00:00"
	case types.AggregationPeriodWeek:
		bucketFmt = "%Y-W%W"
	case types.AggregationPeriodMonth:
		bucketFmt = "%Y-%m-01"
	default: // Day
		bucketFmt = "%Y-%m-%d"
	}

	query := fmt.Sprintf(`
		SELECT
			MIN(timestamp) as bucket_ts,
			COUNT(*) as total_checks,
			SUM(CASE WHEN status IN ('Up', 'UP', 'up', 'Degraded', 'degraded') THEN 1 ELSE 0 END) as successful_checks,
			COALESCE(AVG(CAST(latency_ms AS REAL)), 0) as avg_latency
		FROM monitor_results
		WHERE monitor_uuid = ? AND timestamp >= ? AND timestamp <= ? AND lower(status) IN ('up', 'down', 'degraded', 'error', 'timeout')
		GROUP BY strftime('%s', datetime(timestamp, 'unixepoch'))
		ORDER BY bucket_ts ASC
	`, bucketFmt)

	rows, err := d.db.QueryContext(ctx, query,
		monitorID,
		startTime.Unix(),
		endTime.Unix(),
	)
	if err != nil {
		return nil, fmt.Errorf("failed to query aggregated stats: %w", err)
	}
	defer rows.Close()

	points := []*models.TimeSeriesDataPoint{}
	for rows.Next() {
		var point models.TimeSeriesDataPoint
		var timestamp int64
		var totalChecks, successfulChecks int64
		var avgLatency float64

		err := rows.Scan(&timestamp, &totalChecks, &successfulChecks, &avgLatency)
		if err != nil {
			return nil, fmt.Errorf("failed to scan data point: %w", err)
		}

		point.Timestamp = time.Unix(timestamp, 0)
		point.TotalChecks = totalChecks
		point.SuccessfulChecks = successfulChecks
		if totalChecks > 0 {
			point.UptimePercentage = float64(successfulChecks) / float64(totalChecks) * 100
		}
		point.AverageLatencyMs = avgLatency

		points = append(points, &point)
	}

	return points, rows.Err()
}

// GetGlobalPingSamples retrieves raw monitoring samples for a monitor across nodes within a time range
func (d *LibSQLDatabase) GetGlobalPingSamples(ctx context.Context, monitorID string, startTime, endTime time.Time) ([]*models.MonitoringResult, error) {
	query := `
		SELECT id, monitor_uuid, timestamp, status, latency_ms, status_code,
			peer_id, signature, error_message, created_at
		FROM monitor_results
		WHERE monitor_uuid = ? AND timestamp >= ? AND timestamp <= ? AND lower(status) IN ('up', 'down', 'degraded', 'error', 'timeout')
		ORDER BY timestamp DESC
	`

	rows, err := d.db.QueryContext(ctx, query,
		monitorID,
		startTime.Unix(),
		endTime.Unix(),
	)
	if err != nil {
		return nil, fmt.Errorf("failed to query global ping samples: %w", err)
	}
	defer rows.Close()

	samples := []*models.MonitoringResult{}
	for rows.Next() {
		var result models.MonitoringResult
		var rowID int64
		var statusStr string
		var latencyMs sql.NullInt64
		var statusCode sql.NullInt32
		var errorMsg sql.NullString
		var peerID sql.NullString
		var signature []byte
		var timestamp, createdAt int64

		err := rows.Scan(
			&rowID,
			&result.MonitorID,
			&timestamp,
			&statusStr,
			&latencyMs,
			&statusCode,
			&peerID,
			&signature,
			&errorMsg,
			&createdAt,
		)
		if err != nil {
			return nil, fmt.Errorf("failed to scan global ping sample: %w", err)
		}

		result.ID = fmt.Sprintf("%d", rowID)
		result.Timestamp = time.Unix(timestamp, 0)
		result.CreatedAt = time.Unix(createdAt, 0)

		if latencyMs.Valid {
			result.LatencyMs = latencyMs.Int64
		}
		if peerID.Valid {
			result.MonitorNodeID = peerID.String
		}
		if len(signature) > 0 {
			result.Signature = signature
		}

		// Parse status - Rust stores as Up, Down, Timeout, Error
		result.Status = parseResultStatus(statusStr)

		if statusCode.Valid {
			result.StatusCode = &statusCode.Int32
		}
		if errorMsg.Valid {
			result.ErrorMessage = &errorMsg.String
		}

		samples = append(samples, &result)
	}

	return samples, nil
}

func (d *LibSQLDatabase) GetSetting(ctx context.Context, key string) (string, bool, error) {
	var value string
	err := d.db.QueryRowContext(ctx, "SELECT value FROM settings WHERE key = ?", key).Scan(&value)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return "", false, nil
		}
		return "", false, fmt.Errorf("failed to get setting %s: %w", key, err)
	}
	return value, true, nil
}

func (d *LibSQLDatabase) SetSetting(ctx context.Context, key string, value string) error {
	_, err := d.db.ExecContext(ctx, `
		INSERT INTO settings (key, value, updated_at)
		VALUES (?, ?, ?)
		ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
	`, key, value, time.Now().Unix())
	if err != nil {
		return fmt.Errorf("failed to set setting %s: %w", key, err)
	}
	return nil
}

func (d *LibSQLDatabase) GetNodeIdentity(ctx context.Context) (*models.NodeIdentity, error) {
	nodeName := "Uppe. Node"
	if value, ok, err := d.GetSetting(ctx, "display_name"); err != nil {
		return nil, err
	} else if ok && value != "" {
		nodeName = value
	}

	nodeID, _, err := d.GetSetting(ctx, "node_id")
	if err != nil {
		return nil, err
	}

	var joinedAt int64
	_ = d.db.QueryRowContext(ctx, "SELECT COALESCE(MIN(applied_at), 0) FROM schema_migrations").Scan(&joinedAt)

	var totalChecksPerformed int64
	_ = d.db.QueryRowContext(ctx, "SELECT COUNT(*) FROM monitor_results").Scan(&totalChecksPerformed)

	var totalChecksReceived int64
	_ = d.db.QueryRowContext(ctx, "SELECT COUNT(*) FROM peer_results").Scan(&totalChecksReceived)

	return &models.NodeIdentity{
		NodeID:               nodeID,
		NodeName:             nodeName,
		JoinedNetworkAt:      time.Unix(joinedAt, 0),
		ContributionScore:    1.0,
		TotalChecksPerformed: totalChecksPerformed,
		TotalChecksReceived:  totalChecksReceived,
	}, nil
}

func (d *LibSQLDatabase) GetNetworkStats(ctx context.Context) (*models.NetworkStats, error) {
	nodeIdentity, err := d.GetNodeIdentity(ctx)
	if err != nil {
		return nil, err
	}

	stats := &models.NetworkStats{
		MyNodeID:          nodeIdentity.NodeID,
		ContributionScore: nodeIdentity.ContributionScore,
	}

	var totalPeers, onlinePeers, checksPerformed, checksReceived, bandwidthUsed sql.NullInt64
	err = d.db.QueryRowContext(ctx, `
		SELECT total_peers, online_peers, checks_performed, checks_received, bandwidth_used_mb
		FROM network_stats
		ORDER BY timestamp DESC
		LIMIT 1
	`).Scan(&totalPeers, &onlinePeers, &checksPerformed, &checksReceived, &bandwidthUsed)
	if err != nil && err != sql.ErrNoRows {
		return nil, fmt.Errorf("failed to load network stats: %w", err)
	}

	if errors.Is(err, sql.ErrNoRows) {
		if queryErr := d.db.QueryRowContext(ctx, "SELECT COUNT(*) FROM peers").Scan(&stats.TotalPeers); queryErr != nil {
			return nil, fmt.Errorf("failed to count peers: %w", queryErr)
		}
		if queryErr := d.db.QueryRowContext(ctx, "SELECT COUNT(*) FROM peers WHERE status = 'online'").Scan(&stats.OnlinePeers); queryErr != nil {
			return nil, fmt.Errorf("failed to count online peers: %w", queryErr)
		}
	} else {
		stats.TotalPeers = totalPeers.Int64
		stats.OnlinePeers = onlinePeers.Int64
		stats.MonitoringForOthers = checksPerformed.Int64
		stats.ReceivingFromOthers = checksReceived.Int64
		stats.BandwidthUsedMb = bandwidthUsed.Int64
		stats.ChecksPerHour = checksPerformed.Int64 + checksReceived.Int64
	}

	if stats.TotalPeers > 0 {
		stats.CoveragePercentage = (float64(stats.OnlinePeers) / float64(stats.TotalPeers)) * 100
	}

	if stats.ReceivingFromOthers > 0 {
		stats.ShareRatio = float64(stats.MonitoringForOthers) / float64(stats.ReceivingFromOthers)
	} else {
		stats.ShareRatio = 1.0
	}

	startOfDay := time.Now().UTC().Truncate(24 * time.Hour).Unix()
	_ = d.db.QueryRowContext(ctx,
		"SELECT COUNT(*) FROM monitor_results WHERE timestamp >= ?",
		startOfDay,
	).Scan(&stats.ChecksPerformedToday)
	_ = d.db.QueryRowContext(ctx,
		"SELECT COUNT(*) FROM peer_results WHERE timestamp >= ?",
		startOfDay,
	).Scan(&stats.ChecksReceivedToday)

	if value, ok, getErr := d.GetSetting(ctx, "max_bandwidth_mb_per_day"); getErr != nil {
		return nil, getErr
	} else if ok && value != "" {
		var parsed int64
		if _, scanErr := fmt.Sscan(value, &parsed); scanErr == nil {
			stats.BandwidthLimitMb = parsed
		}
	}
	if stats.BandwidthLimitMb == 0 {
		stats.BandwidthLimitMb = 100
	}

	return stats, nil
}

func (d *LibSQLDatabase) ListPeers(ctx context.Context, query *types.PeerQuery) ([]*models.NetworkPeer, int, error) {
	where := []string{}
	args := []interface{}{}

	if query != nil && query.FilterStatus != "" {
		where = append(where, "status = ?")
		args = append(args, query.FilterStatus)
	}

	whereClause := ""
	if len(where) > 0 {
		whereClause = "WHERE " + joinStrings(where, " AND ")
	}

	countQuery := fmt.Sprintf("SELECT COUNT(*) FROM peers %s", whereClause)
	var total int
	if err := d.db.QueryRowContext(ctx, countQuery, args...).Scan(&total); err != nil {
		return nil, 0, fmt.Errorf("failed to count peers: %w", err)
	}

	orderBy := "last_seen DESC"
	if query != nil {
		switch query.SortBy {
		case "contribution":
			orderBy = "contribution_score DESC, last_seen DESC"
		case "checks":
			orderBy = "checks_per_day DESC, last_seen DESC"
		case "uptime":
			orderBy = "uptime_percentage DESC, last_seen DESC"
		case "last_seen":
			orderBy = "last_seen DESC"
		}
	}

	pageSize := 25
	page := 1
	if query != nil {
		if query.PageSize > 0 {
			pageSize = query.PageSize
		}
		if query.Page > 0 {
			page = query.Page
		}
	}
	offset := (page - 1) * pageSize

	rows, err := d.db.QueryContext(ctx, fmt.Sprintf(`
		SELECT peer_id, location_country, location_region, location_city, status,
			checks_per_day, last_seen, uptime_percentage, contribution_score, joined_at
		FROM peers
		%s
		ORDER BY %s
		LIMIT ? OFFSET ?
	`, whereClause, orderBy), append(args, pageSize, offset)...)
	if err != nil {
		return nil, 0, fmt.Errorf("failed to query peers: %w", err)
	}
	defer rows.Close()

	peers := []*models.NetworkPeer{}
	for rows.Next() {
		peer, scanErr := scanPeer(rows)
		if scanErr != nil {
			return nil, 0, scanErr
		}
		peers = append(peers, peer)
	}

	return peers, total, rows.Err()
}

func (d *LibSQLDatabase) GetPeer(ctx context.Context, peerID string) (*models.NetworkPeer, error) {
	row := d.db.QueryRowContext(ctx, `
		SELECT peer_id, location_country, location_region, location_city, status,
			checks_per_day, last_seen, uptime_percentage, contribution_score, joined_at
		FROM peers
		WHERE peer_id = ?
	`, peerID)

	peer, err := scanPeer(row)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil, fmt.Errorf("peer not found")
		}
		return nil, err
	}
	return peer, nil
}

func (d *LibSQLDatabase) GetNetworkHealth(ctx context.Context) (*models.NetworkHealth, error) {
	health := &models.NetworkHealth{}

	var verifiedCount, peerResultCount int64
	_ = d.db.QueryRowContext(ctx, "SELECT COUNT(*) FROM peer_results WHERE verified = 1").Scan(&verifiedCount)
	_ = d.db.QueryRowContext(ctx, "SELECT COUNT(*) FROM peer_results").Scan(&peerResultCount)
	if peerResultCount > 0 {
		health.ConsensusRate = (float64(verifiedCount) / float64(peerResultCount)) * 100
	}

	var avgLatency sql.NullFloat64
	_ = d.db.QueryRowContext(ctx, `
		SELECT AVG(latency_ms) FROM (
			SELECT latency_ms FROM monitor_results WHERE latency_ms IS NOT NULL
			UNION ALL
			SELECT latency_ms FROM peer_results WHERE latency_ms IS NOT NULL
		)
	`).Scan(&avgLatency)
	if avgLatency.Valid {
		health.AvgLatencyMs = avgLatency.Float64
	}

	var distinctMonitors, totalPeerResults int64
	_ = d.db.QueryRowContext(ctx, "SELECT COUNT(DISTINCT monitor_uuid) FROM peer_results").Scan(&distinctMonitors)
	_ = d.db.QueryRowContext(ctx, "SELECT COUNT(*) FROM peer_results").Scan(&totalPeerResults)
	if distinctMonitors > 0 {
		health.RedundancyFactor = float64(totalPeerResults) / float64(distinctMonitors)
	}

	var peersJoinedLastDay, totalPeers int64
	dayAgo := time.Now().Add(-24 * time.Hour).Unix()
	_ = d.db.QueryRowContext(ctx, "SELECT COUNT(*) FROM peers WHERE joined_at >= ?", dayAgo).Scan(&peersJoinedLastDay)
	_ = d.db.QueryRowContext(ctx, "SELECT COUNT(*) FROM peers").Scan(&totalPeers)
	if totalPeers > 0 {
		health.ChurnRate = (float64(peersJoinedLastDay) / float64(totalPeers)) * 100
	}

	return health, nil
}

func (d *LibSQLDatabase) GetPeerDistribution(ctx context.Context) ([]*models.RegionStat, error) {
	rows, err := d.db.QueryContext(ctx, `
		SELECT
			COALESCE(NULLIF(location_region, ''), NULLIF(location_country, ''), 'Unknown') AS region,
			COUNT(*) AS peer_count,
			SUM(CASE WHEN status = 'online' THEN 1 ELSE 0 END) AS online_count
		FROM peers
		GROUP BY 1
		ORDER BY peer_count DESC, region ASC
	`)
	if err != nil {
		return nil, fmt.Errorf("failed to query peer distribution: %w", err)
	}
	defer rows.Close()

	regions := []*models.RegionStat{}
	for rows.Next() {
		var region models.RegionStat
		if err := rows.Scan(&region.Region, &region.PeerCount, &region.OnlineCount); err != nil {
			return nil, fmt.Errorf("failed to scan region stats: %w", err)
		}
		regions = append(regions, &region)
	}

	return regions, rows.Err()
}

func (d *LibSQLDatabase) CreateStatusPage(ctx context.Context, page *models.StatusPage) error {
	if page.ID == "" {
		page.ID = generateUUID()
	}

	now := time.Now().Unix()
	primaryColor := page.PrimaryColor
	if primaryColor == "" {
		primaryColor = "#3B82F6"
	}

	tx, err := d.db.BeginTx(ctx, nil)
	if err != nil {
		return fmt.Errorf("failed to start transaction: %w", err)
	}
	defer tx.Rollback()

	_, err = tx.ExecContext(ctx, `
		INSERT INTO status_pages (
			uuid, title, slug, description, custom_domain, logo_url, primary_color,
			is_active, visits, user_id, created_at, updated_at
		) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 0, NULL, ?, ?)
	`,
		page.ID,
		page.Title,
		page.Slug,
		page.Description,
		page.CustomDomain,
		page.LogoURL,
		primaryColor,
		boolToInt(page.IsActive),
		now,
		now,
	)
	if err != nil {
		return fmt.Errorf("failed to create status page: %w", err)
	}

	if err := d.replaceStatusPageMonitors(ctx, tx, page.ID, page.MonitorIDs); err != nil {
		return err
	}

	if err := tx.Commit(); err != nil {
		return fmt.Errorf("failed to commit status page creation: %w", err)
	}

	return nil
}

func (d *LibSQLDatabase) GetStatusPage(ctx context.Context, identifier string, bySlug bool) (*models.StatusPage, error) {
	column := "uuid"
	if bySlug {
		column = "slug"
	}

	query := fmt.Sprintf(`
		SELECT uuid, title, slug, description, custom_domain, logo_url, primary_color,
			is_active, visits, created_at, updated_at
		FROM status_pages
		WHERE %s = ?
	`, column)

	row := d.db.QueryRowContext(ctx, query, identifier)
	page, err := scanStatusPage(row)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil, fmt.Errorf("status page not found: %w", sql.ErrNoRows)
		}
		return nil, err
	}

	if err := d.hydrateStatusPage(ctx, page); err != nil {
		return nil, err
	}

	return page, nil
}

func (d *LibSQLDatabase) ListStatusPages(ctx context.Context, page, pageSize int, activeOnly bool) ([]*models.StatusPage, int, error) {
	whereClause := ""
	args := []interface{}{}
	if activeOnly {
		whereClause = "WHERE is_active = 1"
	}

	var total int
	if err := d.db.QueryRowContext(ctx, fmt.Sprintf("SELECT COUNT(*) FROM status_pages %s", whereClause), args...).Scan(&total); err != nil {
		return nil, 0, fmt.Errorf("failed to count status pages: %w", err)
	}

	if pageSize <= 0 {
		pageSize = 25
	}
	if pageSize > 100 {
		pageSize = 100
	}
	if page <= 0 {
		page = 1
	}
	offset := (page - 1) * pageSize

	rows, err := d.db.QueryContext(ctx, fmt.Sprintf(`
		SELECT uuid, title, slug, description, custom_domain, logo_url, primary_color,
			is_active, visits, created_at, updated_at
		FROM status_pages
		%s
		ORDER BY created_at DESC
		LIMIT ? OFFSET ?
	`, whereClause), append(args, pageSize, offset)...)
	if err != nil {
		return nil, 0, fmt.Errorf("failed to query status pages: %w", err)
	}
	defer rows.Close()

	pages := []*models.StatusPage{}
	for rows.Next() {
		page, scanErr := scanStatusPage(rows)
		if scanErr != nil {
			return nil, 0, scanErr
		}
		pages = append(pages, page)
	}

	if err := rows.Err(); err != nil {
		return nil, 0, err
	}
	if err := rows.Close(); err != nil {
		return nil, 0, err
	}
	// Release the cursor before nested queries, including with a one-connection pool.
	for _, page := range pages {
		if err := d.hydrateStatusPage(ctx, page); err != nil {
			return nil, 0, err
		}
	}
	return pages, total, nil
}

func (d *LibSQLDatabase) UpdateStatusPage(ctx context.Context, id string, update *models.StatusPageUpdate) error {
	setClauses := []string{}
	args := []interface{}{}

	if update.Title != nil {
		setClauses = append(setClauses, "title = ?")
		args = append(args, *update.Title)
	}
	if update.Slug != nil {
		setClauses = append(setClauses, "slug = ?")
		args = append(args, *update.Slug)
	}
	if update.Description != nil {
		setClauses = append(setClauses, "description = ?")
		args = append(args, *update.Description)
	}
	if update.IsActive != nil {
		setClauses = append(setClauses, "is_active = ?")
		args = append(args, boolToInt(*update.IsActive))
	}
	if update.LogoURL != nil {
		setClauses = append(setClauses, "logo_url = ?")
		args = append(args, *update.LogoURL)
	}
	if update.PrimaryColor != nil {
		setClauses = append(setClauses, "primary_color = ?")
		args = append(args, *update.PrimaryColor)
	}

	tx, err := d.db.BeginTx(ctx, nil)
	if err != nil {
		return fmt.Errorf("failed to start transaction: %w", err)
	}
	defer tx.Rollback()

	if len(setClauses) > 0 {
		setClauses = append(setClauses, "updated_at = ?")
		args = append(args, time.Now().Unix(), id)

		query := fmt.Sprintf("UPDATE status_pages SET %s WHERE uuid = ?", joinStrings(setClauses, ", "))
		if _, err := tx.ExecContext(ctx, query, args...); err != nil {
			return fmt.Errorf("failed to update status page: %w", err)
		}
	}

	if update.ReplaceMonitorIDs {
		if err := d.replaceStatusPageMonitors(ctx, tx, id, update.MonitorIDs); err != nil {
			return err
		}
	}

	if err := tx.Commit(); err != nil {
		return fmt.Errorf("failed to commit status page update: %w", err)
	}
	return nil
}

func (d *LibSQLDatabase) DeleteStatusPage(ctx context.Context, id string) error {
	_, err := d.db.ExecContext(ctx, "DELETE FROM status_pages WHERE uuid = ?", id)
	if err != nil {
		return fmt.Errorf("failed to delete status page: %w", err)
	}
	return nil
}

func (d *LibSQLDatabase) AddStatusPageVisits(ctx context.Context, id string, count int64) error {
	_, err := d.db.ExecContext(ctx, `
		UPDATE status_pages
		SET visits = visits + ?
		WHERE uuid = ?
	`, count, id)
	if err != nil {
		return fmt.Errorf("failed to record status page visit: %w", err)
	}
	return nil
}

func (d *LibSQLDatabase) replaceStatusPageMonitors(ctx context.Context, tx *sql.Tx, statusPageID string, monitorIDs []string) error {
	if _, err := tx.ExecContext(ctx, "DELETE FROM status_page_monitors WHERE status_page_id = ?", statusPageID); err != nil {
		return fmt.Errorf("failed to clear status page monitors: %w", err)
	}
	for index, monitorID := range monitorIDs {
		if _, err := tx.ExecContext(ctx, `
			INSERT INTO status_page_monitors (status_page_id, monitor_uuid, display_order)
			VALUES (?, ?, ?)
		`, statusPageID, monitorID, index); err != nil {
			return fmt.Errorf("failed to link monitor to status page: %w", err)
		}
	}
	return nil
}

type statusPageScanner interface {
	Scan(dest ...interface{}) error
}

func scanStatusPage(scanner statusPageScanner) (*models.StatusPage, error) {
	var page models.StatusPage
	var customDomain, logoURL sql.NullString
	var isActive int
	var createdAt, updatedAt int64

	if err := scanner.Scan(
		&page.ID,
		&page.Title,
		&page.Slug,
		&page.Description,
		&customDomain,
		&logoURL,
		&page.PrimaryColor,
		&isActive,
		&page.Visits,
		&createdAt,
		&updatedAt,
	); err != nil {
		return nil, fmt.Errorf("failed to scan status page: %w", err)
	}

	page.IsActive = isActive == 1
	page.CreatedAt = time.Unix(createdAt, 0)
	page.UpdatedAt = time.Unix(updatedAt, 0)
	if customDomain.Valid {
		page.CustomDomain = &customDomain.String
	}
	if logoURL.Valid {
		page.LogoURL = &logoURL.String
	}
	if page.PrimaryColor == "" {
		page.PrimaryColor = "#3B82F6"
	}

	return &page, nil
}

func (d *LibSQLDatabase) hydrateStatusPage(ctx context.Context, page *models.StatusPage) error {
	monitorRows, err := d.db.QueryContext(ctx, `
		SELECT monitor_uuid
		FROM status_page_monitors
		WHERE status_page_id = ?
		ORDER BY display_order ASC, monitor_uuid ASC
	`, page.ID)
	if err != nil {
		return fmt.Errorf("failed to load status page monitors: %w", err)
	}
	defer monitorRows.Close()

	monitorIDs := []string{}
	for monitorRows.Next() {
		var monitorID string
		if err := monitorRows.Scan(&monitorID); err != nil {
			return fmt.Errorf("failed to scan status page monitor: %w", err)
		}
		monitorIDs = append(monitorIDs, monitorID)
	}
	if err := monitorRows.Err(); err != nil {
		return err
	}
	page.MonitorIDs = monitorIDs
	// Availability is calculated by the public projection from current local checks.
	// Management metadata does not infer health from an empty history.
	return nil
}

type peerScanner interface {
	Scan(dest ...interface{}) error
}

func scanPeer(scanner peerScanner) (*models.NetworkPeer, error) {
	var peer models.NetworkPeer
	var country, region, city sql.NullString
	var lastSeen, joinedAt int64

	if err := scanner.Scan(
		&peer.PeerID,
		&country,
		&region,
		&city,
		&peer.Status,
		&peer.ChecksPerDay,
		&lastSeen,
		&peer.UptimePercentage,
		&peer.ContributionScore,
		&joinedAt,
	); err != nil {
		return nil, fmt.Errorf("failed to scan peer: %w", err)
	}

	peer.LastSeen = time.Unix(lastSeen, 0)
	peer.JoinedAt = time.Unix(joinedAt, 0)
	peer.Location = &commonv1.Location{
		Country: country.String,
		Region:  region.String,
		City:    city.String,
	}

	return &peer, nil
}
