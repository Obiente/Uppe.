package result

import (
	"context"
	"fmt"
	"time"

	"connectrpc.com/connect"
	"go.uber.org/zap"

	resultv1 "github.com/Obiente/Uppe/apps/server/gen/result/v1"
	"github.com/Obiente/Uppe/apps/server/gen/result/v1/resultv1connect"
	"github.com/Obiente/Uppe/apps/server/internal/db"
	"github.com/Obiente/Uppe/apps/server/internal/db/types"
)

// ResultService implements the ResultService ConnectRPC service
type ResultService struct {
	logger   *zap.Logger
	database db.Database
}

// Ensure ResultService implements the interface
var _ resultv1connect.ResultServiceHandler = (*ResultService)(nil)

func NewResultService(logger *zap.Logger, database db.Database) *ResultService {
	return &ResultService{
		logger:   logger,
		database: database,
	}
}

func (s *ResultService) GetResults(
	ctx context.Context,
	req *connect.Request[resultv1.GetResultsRequest],
) (*connect.Response[resultv1.GetResultsResponse], error) {
	s.logger.Info("GetResults called", zap.String("monitor_id", req.Msg.MonitorId))

	query := &types.ResultQuery{
		MonitorID: req.Msg.MonitorId,
		StartTime: time.Unix(req.Msg.StartTime, 0),
		EndTime:   time.Unix(req.Msg.EndTime, 0),
		Limit:     int(req.Msg.Limit),
		Offset:    int(req.Msg.Offset),
	}
	if req.Msg.EndTime == 0 {
		query.EndTime = time.Now()
	}
	if req.Msg.StartTime == 0 {
		query.StartTime = query.EndTime.Add(-24 * time.Hour)
	}
	if query.StartTime.After(query.EndTime) || query.EndTime.Sub(query.StartTime) > 366*24*time.Hour {
		return nil, connect.NewError(connect.CodeInvalidArgument, fmt.Errorf("choose a time range of at most 366 days"))
	}

	results, total, err := s.database.GetResults(ctx, query)
	if err != nil {
		s.logger.Error("Failed to get results", zap.Error(err))
		return nil, connect.NewError(connect.CodeInternal, fmt.Errorf("failed to get results: %w", err))
	}

	protoResults := make([]*resultv1.MonitoringResult, len(results))
	for i, r := range results {
		protoResults[i] = r.ToProto()
	}

	return connect.NewResponse(&resultv1.GetResultsResponse{
		Results: protoResults,
		Total:   int32(total),
	}), nil
}

func (s *ResultService) GetAggregatedStats(
	ctx context.Context,
	req *connect.Request[resultv1.GetAggregatedStatsRequest],
) (*connect.Response[resultv1.AggregatedStats], error) {
	s.logger.Info("GetAggregatedStats called", zap.String("monitor_id", req.Msg.MonitorId))

	var period types.AggregationPeriod
	switch req.Msg.Period {
	case resultv1.AggregationPeriod_AGGREGATION_PERIOD_HOUR:
		period = types.AggregationPeriodHour
	case resultv1.AggregationPeriod_AGGREGATION_PERIOD_DAY:
		period = types.AggregationPeriodDay
	case resultv1.AggregationPeriod_AGGREGATION_PERIOD_WEEK:
		period = types.AggregationPeriodWeek
	case resultv1.AggregationPeriod_AGGREGATION_PERIOD_MONTH:
		period = types.AggregationPeriodMonth
	default:
		period = types.AggregationPeriodDay
	}

	points, err := s.database.GetAggregatedStats(ctx,
		req.Msg.MonitorId,
		time.Unix(req.Msg.StartTime, 0),
		time.Unix(req.Msg.EndTime, 0),
		period,
	)
	if err != nil {
		s.logger.Error("Failed to get aggregated stats", zap.Error(err))
		return nil, connect.NewError(connect.CodeInternal, fmt.Errorf("failed to get aggregated stats: %w", err))
	}

	protoPoints := make([]*resultv1.TimeSeriesDataPoint, len(points))
	for i, p := range points {
		protoPoints[i] = p.ToProto()
	}

	return connect.NewResponse(&resultv1.AggregatedStats{
		DataPoints: protoPoints,
	}), nil
}

func (s *ResultService) StreamResults(
	ctx context.Context,
	req *connect.Request[resultv1.StreamResultsRequest],
	stream *connect.ServerStream[resultv1.MonitoringResult],
) error {
	s.logger.Info("StreamResults called", zap.String("monitor_id", req.Msg.MonitorId))

	return connect.NewError(connect.CodeUnimplemented, fmt.Errorf("result streaming is not available"))
}

func (s *ResultService) GetGlobalPing(
	ctx context.Context,
	req *connect.Request[resultv1.GetGlobalPingRequest],
) (*connect.Response[resultv1.GetGlobalPingResponse], error) {
	s.logger.Info("GetGlobalPing called", zap.String("monitor_id", req.Msg.MonitorId))

	samples, err := s.database.GetGlobalPingSamples(ctx,
		req.Msg.MonitorId,
		time.Unix(req.Msg.StartTime, 0),
		time.Unix(req.Msg.EndTime, 0),
	)
	if err != nil {
		s.logger.Error("Failed to get global ping samples", zap.Error(err))
		return nil, connect.NewError(connect.CodeInternal, fmt.Errorf("failed to get global ping samples: %w", err))
	}

	protoSamples := make([]*resultv1.GlobalPingSample, len(samples))
	for i, r := range samples {
		protoSamples[i] = &resultv1.GlobalPingSample{
			PeerId:    r.MonitorNodeID,
			Location:  r.Location,
			LatencyMs: int32(r.LatencyMs),
			Status:    r.Status,
			Timestamp: r.Timestamp.Unix(),
		}
	}

	return connect.NewResponse(&resultv1.GetGlobalPingResponse{Samples: protoSamples}), nil
}
