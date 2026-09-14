package models

import (
	"time"

	commonv1 "github.com/Obiente/Uppe/apps/server/gen/common/v1"
)

// NetworkStats represents aggregate P2P network metrics.
type NetworkStats struct {
	TotalPeers           int64
	OnlinePeers          int64
	ChecksPerHour        int64
	CoveragePercentage   float64
	ShareRatio           float64
	MonitoringForOthers  int64
	ReceivingFromOthers  int64
	ContributionScore    float64
	MyNodeID             string
	BandwidthUsedMb      int64
	BandwidthLimitMb     int64
	ChecksPerformedToday int64
	ChecksReceivedToday  int64
}

// NetworkPeer represents a peer in the distributed network.
type NetworkPeer struct {
	PeerID            string
	Location          *commonv1.Location
	Status            string
	ChecksPerDay      int64
	LastSeen          time.Time
	UptimePercentage  float64
	ContributionScore float64
	JoinedAt          time.Time
}

// NetworkHealth captures coarse health indicators derived from replicated data.
type NetworkHealth struct {
	ConsensusRate    float64
	AvgLatencyMs     float64
	RedundancyFactor float64
	ChurnRate        float64
}

// RegionStat aggregates peer counts by region.
type RegionStat struct {
	Region       string
	PeerCount    int64
	OnlineCount  int64
	AvgLatencyMs float64
}
