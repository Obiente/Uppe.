package db

import (
	"context"
	"time"

	"github.com/Obiente/Uppe/apps/server/internal/db/types"
	"github.com/Obiente/Uppe/apps/server/internal/models"
)

// Database defines the interface for database operations
// This allows us to support multiple database backends (LibSQL, PostgreSQL, TimescaleDB)
type Database interface {
	// Monitor operations
	CreateMonitor(ctx context.Context, monitor *models.Monitor) error
	GetMonitor(ctx context.Context, id string) (*models.Monitor, error)
	ListMonitors(ctx context.Context, filters *types.MonitorFilters) ([]*models.Monitor, int, error)
	UpdateMonitor(ctx context.Context, id string, updates *models.MonitorUpdate) error
	DeleteMonitor(ctx context.Context, id string) error
	GetMonitorStats(ctx context.Context, monitorID string, startTime, endTime time.Time) (*models.MonitorStats, error)

	// Results are written only by Rust.
	GetResults(ctx context.Context, query *types.ResultQuery) ([]*models.MonitoringResult, int, error)
	GetAggregatedStats(ctx context.Context, monitorID string, startTime, endTime time.Time, period types.AggregationPeriod) ([]*models.TimeSeriesDataPoint, error)

	// Global ping samples across nodes/locations for a monitor
	GetGlobalPingSamples(ctx context.Context, monitorID string, startTime, endTime time.Time) ([]*models.MonitoringResult, error)

	// Settings operations
	GetSetting(ctx context.Context, key string) (string, bool, error)
	SetSetting(ctx context.Context, key string, value string) error
	GetNodeIdentity(ctx context.Context) (*models.NodeIdentity, error)

	// Network operations
	GetNetworkStats(ctx context.Context) (*models.NetworkStats, error)
	ListPeers(ctx context.Context, query *types.PeerQuery) ([]*models.NetworkPeer, int, error)
	GetPeer(ctx context.Context, peerID string) (*models.NetworkPeer, error)
	GetNetworkHealth(ctx context.Context) (*models.NetworkHealth, error)
	GetPeerDistribution(ctx context.Context) ([]*models.RegionStat, error)

	// Status page operations
	CreateStatusPage(ctx context.Context, page *models.StatusPage) error
	GetStatusPage(ctx context.Context, identifier string, bySlug bool) (*models.StatusPage, error)
	ListStatusPages(ctx context.Context, page, pageSize int, activeOnly bool) ([]*models.StatusPage, int, error)
	UpdateStatusPage(ctx context.Context, id string, update *models.StatusPageUpdate) error
	DeleteStatusPage(ctx context.Context, id string) error
	AddStatusPageVisits(ctx context.Context, id string, count int64) error

	// Health check
	Ping(ctx context.Context) error
	Close() error
}
