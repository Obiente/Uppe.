package settings

import (
	"connectrpc.com/connect"
	"context"
	"fmt"
	settingsv1 "github.com/Obiente/Uppe/apps/server/gen/settings/v1"
	"github.com/Obiente/Uppe/apps/server/internal/db"
	"go.uber.org/zap"
)

type SettingsService struct {
	logger   *zap.Logger
	database db.Database
}

func NewSettingsService(logger *zap.Logger, database db.Database) *SettingsService {
	return &SettingsService{logger, database}
}

// Runtime settings belong to the Rust configuration until a live configuration protocol exists.
func (s *SettingsService) GetSettings(context.Context, *connect.Request[settingsv1.GetSettingsRequest]) (*connect.Response[settingsv1.Settings], error) {
	return nil, connect.NewError(connect.CodeUnimplemented, fmt.Errorf("runtime settings are configured in config.toml"))
}
func (s *SettingsService) UpdateSettings(context.Context, *connect.Request[settingsv1.UpdateSettingsRequest]) (*connect.Response[settingsv1.Settings], error) {
	return nil, connect.NewError(connect.CodeUnimplemented, fmt.Errorf("edit config.toml and restart the service to change runtime settings"))
}
func (s *SettingsService) GetNodeIdentity(
	ctx context.Context,
	_ *connect.Request[settingsv1.GetNodeIdentityRequest],
) (*connect.Response[settingsv1.NodeIdentity], error) {
	identity, err := s.database.GetNodeIdentity(ctx)
	if err != nil {
		s.logger.Error("Failed to load node identity", zap.Error(err))
		return nil, connect.NewError(connect.CodeInternal, fmt.Errorf("failed to load node identity"))
	}

	return connect.NewResponse(&settingsv1.NodeIdentity{
		NodeId:               identity.NodeID,
		NodeName:             identity.NodeName,
		JoinedNetworkAt:      identity.JoinedNetworkAt.Unix(),
		ContributionScore:    identity.ContributionScore,
		TotalChecksPerformed: identity.TotalChecksPerformed,
		TotalChecksReceived:  identity.TotalChecksReceived,
	}), nil
}
