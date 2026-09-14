package models

import (
	commonv1 "github.com/Obiente/Uppe/apps/server/gen/common/v1"
	resultv1 "github.com/Obiente/Uppe/apps/server/gen/result/v1"
	"time"
)

type MonitoringResult struct {
	ID, MonitorID, MonitorNodeID string
	Timestamp, CreatedAt         time.Time
	Status                       resultv1.ResultStatus
	LatencyMs                    int64
	StatusCode                   *int32
	ErrorMessage                 *string
	Signature                    []byte
	Location                     *commonv1.Location
}

func (r *MonitoringResult) ToProto() *resultv1.MonitoringResult {
	return &resultv1.MonitoringResult{Id: r.ID, MonitorId: r.MonitorID, MonitorNodeId: r.MonitorNodeID,
		Timestamp: r.Timestamp.Unix(), CreatedAt: r.CreatedAt.Unix(), Status: r.Status, LatencyMs: r.LatencyMs,
		StatusCode: r.StatusCode, ErrorMessage: r.ErrorMessage, Signature: r.Signature, Location: r.Location}
}

type TimeSeriesDataPoint struct {
	Timestamp                          time.Time
	TotalChecks, SuccessfulChecks      int64
	UptimePercentage, AverageLatencyMs float64
}

func (p *TimeSeriesDataPoint) ToProto() *resultv1.TimeSeriesDataPoint {
	return &resultv1.TimeSeriesDataPoint{Timestamp: p.Timestamp.Unix(), TotalChecks: p.TotalChecks,
		SuccessfulChecks: p.SuccessfulChecks, UptimePercentage: p.UptimePercentage, AverageLatencyMs: p.AverageLatencyMs}
}

type NodeIdentity struct {
	NodeID, NodeName                          string
	JoinedNetworkAt                           time.Time
	ContributionScore                         float64
	TotalChecksPerformed, TotalChecksReceived int64
}
