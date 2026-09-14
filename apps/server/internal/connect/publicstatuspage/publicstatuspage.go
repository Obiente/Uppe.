package publicstatuspage

import (
	"connectrpc.com/connect"
	"context"
	"database/sql"
	"errors"
	pagev1 "github.com/Obiente/Uppe/apps/server/gen/publicstatuspage/v1"
	resultv1 "github.com/Obiente/Uppe/apps/server/gen/result/v1"
	"github.com/Obiente/Uppe/apps/server/internal/db"
	"github.com/Obiente/Uppe/apps/server/internal/db/types"
	"go.uber.org/zap"
	"time"
)

// Public reads deliberately expose a projection, never operator monitor targets,
// headers, bodies, peer identities, or the management services.
type Service struct {
	database db.Database
	logger   *zap.Logger
}

func New(d db.Database, l *zap.Logger) *Service { return &Service{d, l} }
func (s *Service) GetPublicStatusPage(ctx context.Context, r *connect.Request[pagev1.GetPublicStatusPageRequest]) (*connect.Response[pagev1.PublicStatusPage], error) {
	if len(r.Msg.Slug) < 3 || len(r.Msg.Slug) > 63 {
		return nil, connect.NewError(connect.CodeNotFound, errors.New("page not found"))
	}
	page, err := s.database.GetStatusPage(ctx, r.Msg.Slug, true)
	if err != nil && !errors.Is(err, sql.ErrNoRows) {
		s.logger.Error("Public page read failed", zap.Error(err))
		return nil, connect.NewError(connect.CodeUnavailable, errors.New("page temporarily unavailable"))
	}
	if err != nil || !page.IsActive {
		return nil, connect.NewError(connect.CodeNotFound, errors.New("page not found"))
	}
	if len(page.MonitorIDs) > 100 {
		return nil, connect.NewError(connect.CodeResourceExhausted, errors.New("too many monitors"))
	}
	now := time.Now()
	start := now.Add(-24 * time.Hour)
	out := &pagev1.PublicStatusPage{Title: page.Title, Slug: page.Slug, Description: page.Description, LogoUrl: stringValue(page.LogoURL), PrimaryColor: page.PrimaryColor, UpdatedAt: page.UpdatedAt.Unix()}
	for _, id := range page.MonitorIDs {
		summary := &pagev1.PublicMonitorSummary{Id: id, Name: "Unavailable service", Status: resultv1.ResultStatus_RESULT_STATUS_UNSPECIFIED}
		monitor, err := s.database.GetMonitor(ctx, id)
		if err == nil {
			summary.Name = monitor.Name
			if monitor.PublicDisplayName != nil && *monitor.PublicDisplayName != "" {
				summary.Name = *monitor.PublicDisplayName
			}
			results, _, readErr := s.database.GetResults(ctx, &types.ResultQuery{MonitorID: id, StartTime: start, EndTime: now, Limit: 1})
			if readErr == nil && len(results) > 0 {
				summary.LastChecked = results[0].Timestamp.Unix()
				// A formerly healthy result must not stay green when monitoring stops.
				freshness := time.Duration(max(int64(monitor.IntervalSeconds)*2, int64(monitor.TimeoutSeconds)+30)) * time.Second
				if monitor.Enabled && results[0].Timestamp.After(monitor.UpdatedAt) && now.Sub(results[0].Timestamp) <= freshness {
					summary.Status = results[0].Status
				}
			}
			stats, statsErr := s.database.GetMonitorStats(ctx, id, start, now)
			if statsErr == nil {
				summary.TotalChecks = stats.TotalChecks
				summary.UptimePercentage = stats.UptimePercentage
				summary.AverageLatencyMs = stats.AverageLatencyMs
			}
			history, historyErr := s.database.GetAggregatedStats(ctx, id, start, now, types.AggregationPeriodHour)
			if historyErr == nil {
				for _, p := range history {
					summary.History = append(summary.History, p.ToProto())
				}
			}
			if readErr != nil || statsErr != nil || historyErr != nil {
				s.logger.Warn("Public monitor data unavailable", zap.String("monitor_id", id))
				summary.Status = resultv1.ResultStatus_RESULT_STATUS_UNSPECIFIED
			}
		}
		out.Monitors = append(out.Monitors, summary)
	}
	// Visit counters are best-effort and must never determine availability.
	if err := s.database.RecordStatusPageVisit(ctx, page.ID); err != nil {
		s.logger.Warn("Visit counter unavailable", zap.Error(err))
	}
	return connect.NewResponse(out), nil
}
func stringValue(s *string) string {
	if s == nil {
		return ""
	}
	return *s
}
