package network

import (
	"context"
	"fmt"
	"strings"

	"connectrpc.com/connect"
	"go.uber.org/zap"

	commonv1 "github.com/Obiente/Uppe/apps/server/gen/common/v1"
	networkv1 "github.com/Obiente/Uppe/apps/server/gen/network/v1"
	"github.com/Obiente/Uppe/apps/server/gen/network/v1/networkv1connect"
	"github.com/Obiente/Uppe/apps/server/internal/db"
	dbtypes "github.com/Obiente/Uppe/apps/server/internal/db/types"
	"github.com/Obiente/Uppe/apps/server/internal/models"
)

type NetworkService struct {
	logger   *zap.Logger
	database db.Database
}

var _ networkv1connect.NetworkServiceHandler = (*NetworkService)(nil)

func NewNetworkService(logger *zap.Logger, database db.Database) *NetworkService {
	return &NetworkService{
		logger:   logger,
		database: database,
	}
}

func (s *NetworkService) GetNetworkStats(
	ctx context.Context,
	_ *connect.Request[networkv1.GetNetworkStatsRequest],
) (*connect.Response[networkv1.NetworkStats], error) {
	stats, err := s.database.GetNetworkStats(ctx)
	if err != nil {
		s.logger.Error("Failed to load network stats", zap.Error(err))
		return nil, connect.NewError(connect.CodeInternal, fmt.Errorf("failed to load network stats: %w", err))
	}

	return connect.NewResponse(&networkv1.NetworkStats{
		TotalPeers:           stats.TotalPeers,
		OnlinePeers:          stats.OnlinePeers,
		ChecksPerHour:        stats.ChecksPerHour,
		CoveragePercentage:   stats.CoveragePercentage,
		ShareRatio:           stats.ShareRatio,
		MonitoringForOthers:  stats.MonitoringForOthers,
		ReceivingFromOthers:  stats.ReceivingFromOthers,
		ContributionScore:    stats.ContributionScore,
		MyNodeId:             stats.MyNodeID,
		BandwidthUsedMb:      stats.BandwidthUsedMb,
		BandwidthLimitMb:     stats.BandwidthLimitMb,
		ChecksPerformedToday: stats.ChecksPerformedToday,
		ChecksReceivedToday:  stats.ChecksReceivedToday,
	}), nil
}

func (s *NetworkService) ListPeers(
	ctx context.Context,
	req *connect.Request[networkv1.ListPeersRequest],
) (*connect.Response[networkv1.ListPeersResponse], error) {
	filterStatus := ""
	if req.Msg.FilterStatus != networkv1.PeerStatus_PEER_STATUS_UNSPECIFIED {
		filterStatus = peerStatusToDB(req.Msg.FilterStatus)
	}

	peers, total, err := s.database.ListPeers(ctx, &dbtypes.PeerQuery{
		Page:         int(req.Msg.Page),
		PageSize:     int(req.Msg.PageSize),
		SortBy:       req.Msg.SortBy,
		FilterStatus: filterStatus,
	})
	if err != nil {
		s.logger.Error("Failed to list peers", zap.Error(err))
		return nil, connect.NewError(connect.CodeInternal, fmt.Errorf("failed to list peers: %w", err))
	}

	response := &networkv1.ListPeersResponse{
		Peers:    make([]*networkv1.Peer, 0, len(peers)),
		Total:    int32(total),
		Page:     req.Msg.Page,
		PageSize: req.Msg.PageSize,
	}
	for _, peer := range peers {
		response.Peers = append(response.Peers, peerToProto(peer))
	}

	return connect.NewResponse(response), nil
}

func (s *NetworkService) GetPeer(
	ctx context.Context,
	req *connect.Request[networkv1.GetPeerRequest],
) (*connect.Response[networkv1.Peer], error) {
	peer, err := s.database.GetPeer(ctx, req.Msg.PeerId)
	if err != nil {
		s.logger.Error("Failed to load peer", zap.String("peer_id", req.Msg.PeerId), zap.Error(err))
		return nil, connect.NewError(connect.CodeNotFound, fmt.Errorf("failed to load peer: %w", err))
	}

	return connect.NewResponse(peerToProto(peer)), nil
}

func (s *NetworkService) GetNetworkHealth(
	ctx context.Context,
	_ *connect.Request[networkv1.GetNetworkHealthRequest],
) (*connect.Response[networkv1.NetworkHealth], error) {
	health, err := s.database.GetNetworkHealth(ctx)
	if err != nil {
		s.logger.Error("Failed to load network health", zap.Error(err))
		return nil, connect.NewError(connect.CodeInternal, fmt.Errorf("failed to load network health: %w", err))
	}

	return connect.NewResponse(&networkv1.NetworkHealth{
		ConsensusRate:    health.ConsensusRate,
		AvgLatencyMs:     health.AvgLatencyMs,
		RedundancyFactor: health.RedundancyFactor,
		ChurnRate:        health.ChurnRate,
	}), nil
}

func (s *NetworkService) GetPeerDistribution(
	ctx context.Context,
	_ *connect.Request[networkv1.GetPeerDistributionRequest],
) (*connect.Response[networkv1.PeerDistribution], error) {
	regions, err := s.database.GetPeerDistribution(ctx)
	if err != nil {
		s.logger.Error("Failed to load peer distribution", zap.Error(err))
		return nil, connect.NewError(connect.CodeInternal, fmt.Errorf("failed to load peer distribution: %w", err))
	}

	response := &networkv1.PeerDistribution{
		Regions: make([]*networkv1.RegionStats, 0, len(regions)),
	}
	for _, region := range regions {
		response.Regions = append(response.Regions, &networkv1.RegionStats{
			Region:       region.Region,
			PeerCount:    region.PeerCount,
			OnlineCount:  region.OnlineCount,
			AvgLatencyMs: region.AvgLatencyMs,
		})
	}
	return connect.NewResponse(response), nil
}

func peerToProto(peer *models.NetworkPeer) *networkv1.Peer {
	location := peer.Location
	if location == nil {
		location = &commonv1.Location{}
	}

	return &networkv1.Peer{
		PeerId:            peer.PeerID,
		Location:          location,
		Status:            peerStatusFromDB(peer.Status),
		ChecksPerDay:      peer.ChecksPerDay,
		LastSeen:          peer.LastSeen.Unix(),
		UptimePercentage:  peer.UptimePercentage,
		ContributionScore: peer.ContributionScore,
		JoinedAt:          peer.JoinedAt.Unix(),
	}
}

func peerStatusFromDB(status string) networkv1.PeerStatus {
	switch strings.ToLower(status) {
	case "online":
		return networkv1.PeerStatus_PEER_STATUS_ONLINE
	case "slow":
		return networkv1.PeerStatus_PEER_STATUS_SLOW
	case "offline":
		return networkv1.PeerStatus_PEER_STATUS_OFFLINE
	default:
		return networkv1.PeerStatus_PEER_STATUS_UNSPECIFIED
	}
}

func peerStatusToDB(status networkv1.PeerStatus) string {
	switch status {
	case networkv1.PeerStatus_PEER_STATUS_ONLINE:
		return "online"
	case networkv1.PeerStatus_PEER_STATUS_SLOW:
		return "slow"
	case networkv1.PeerStatus_PEER_STATUS_OFFLINE:
		return "offline"
	default:
		return ""
	}
}
