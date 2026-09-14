package types

import "time"

// MonitorFilters for listing monitors
type MonitorFilters struct {
	UserID     *string
	Enabled    *bool
	Visibility *string // Optional filter: "Public", "Private", "Internal"
	Page       int
	PageSize   int
}

// ResultQuery for querying results
type ResultQuery struct {
	MonitorID string
	StartTime time.Time
	EndTime   time.Time
	Limit     int
	Offset    int
}

// PeerQuery for querying peers in the network view
type PeerQuery struct {
	Page         int
	PageSize     int
	SortBy       string
	FilterStatus string
}

// AggregationPeriod for time-series aggregation
type AggregationPeriod string

const (
	AggregationPeriodHour  AggregationPeriod = "hour"
	AggregationPeriodDay   AggregationPeriod = "day"
	AggregationPeriodWeek  AggregationPeriod = "week"
	AggregationPeriodMonth AggregationPeriod = "month"
)
