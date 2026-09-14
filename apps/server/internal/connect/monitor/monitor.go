package monitor

import (
	"context"
	"database/sql"
	"errors"
	"fmt"
	"net"
	"net/url"
	"strconv"
	"strings"
	"time"

	"connectrpc.com/connect"
	monitorv1 "github.com/Obiente/Uppe/apps/server/gen/monitor/v1"
	"github.com/Obiente/Uppe/apps/server/internal/db"
	"github.com/Obiente/Uppe/apps/server/internal/db/types"
	"github.com/Obiente/Uppe/apps/server/internal/models"
	"go.uber.org/zap"
	"google.golang.org/protobuf/types/known/emptypb"
)

type MonitorService struct {
	logger   *zap.Logger
	database db.Database
}

func NewMonitorService(l *zap.Logger, d db.Database) *MonitorService { return &MonitorService{l, d} }

// Validate at the API boundary; the Rust executor independently checks resolved addresses.
func validate(m *models.Monitor) error {
	if strings.TrimSpace(m.Name) == "" || len(m.Name) > 200 {
		return fmt.Errorf("name must contain 1 to 200 characters")
	}
	if len(m.URL) > 2048 {
		return fmt.Errorf("target is too long")
	}
	if m.IntervalSeconds < 10 || m.IntervalSeconds > 86400 {
		return fmt.Errorf("interval must be between 10 and 86400 seconds")
	}
	if m.TimeoutSeconds < 1 || m.TimeoutSeconds > 300 || m.TimeoutSeconds > m.IntervalSeconds {
		return fmt.Errorf("timeout must be between 1 and 300 seconds and no longer than the interval")
	}
	switch m.Type {
	case monitorv1.MonitorType_MONITOR_TYPE_HTTP:
		u, e := url.Parse(m.URL)
		if e != nil || u.Hostname() == "" || (u.Scheme != "http" && u.Scheme != "https") || u.User != nil || u.Fragment != "" {
			return fmt.Errorf("target must be an HTTP or HTTPS URL without credentials or fragment")
		}
	case monitorv1.MonitorType_MONITOR_TYPE_TCP:
		h, p, e := net.SplitHostPort(m.URL)
		if e != nil || h == "" {
			return fmt.Errorf("TCP target must be host:port or [IPv6]:port")
		}
		n, e := strconv.Atoi(p)
		if e != nil || n < 1 || n > 65535 {
			return fmt.Errorf("invalid TCP port")
		}
	default:
		return fmt.Errorf("choose an HTTP or TCP monitor")
	}
	if m.Visibility != models.MonitorVisibilityInternal && (len(m.Headers) > 0 || m.Body != "") {
		return fmt.Errorf("headers and request bodies require an internal monitor")
	}
	// Custom HTTP options are not implemented by the execution contract yet. Fail instead of silently ignoring them.
	if len(m.Headers) > 0 || m.Body != "" || len(m.ExpectedStatusCodes) > 0 {
		return fmt.Errorf("custom HTTP headers, bodies and status codes are not supported yet")
	}
	return nil
}
func (s *MonitorService) CreateMonitor(ctx context.Context, r *connect.Request[monitorv1.CreateMonitorRequest]) (*connect.Response[monitorv1.Monitor], error) {
	p := r.Msg
	if _, ok := monitorv1.MonitorVisibility_name[int32(p.Visibility)]; !ok {
		return nil, connect.NewError(connect.CodeInvalidArgument, fmt.Errorf("invalid visibility"))
	}
	m := &models.Monitor{Name: p.Name, URL: p.Url, Type: p.Type, IntervalSeconds: p.IntervalSeconds, TimeoutSeconds: p.TimeoutSeconds, Enabled: p.Enabled, Visibility: models.VisibilityFromProto(p.Visibility), Headers: p.Headers, Body: p.Body, ExpectedStatusCodes: p.ExpectedStatusCodes}
	if p.PublicDomain != "" {
		m.PublicDomain = &p.PublicDomain
	}
	if p.PublicDisplayName != "" {
		m.PublicDisplayName = &p.PublicDisplayName
	}
	if err := validate(m); err != nil {
		return nil, connect.NewError(connect.CodeInvalidArgument, err)
	}
	if err := s.database.CreateMonitor(ctx, m); err != nil {
		return nil, s.failure(err)
	}
	return s.GetMonitor(ctx, connect.NewRequest(&monitorv1.GetMonitorRequest{Id: m.ID}))
}
func (s *MonitorService) GetMonitor(ctx context.Context, r *connect.Request[monitorv1.GetMonitorRequest]) (*connect.Response[monitorv1.Monitor], error) {
	m, err := s.database.GetMonitor(ctx, r.Msg.Id)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil, connect.NewError(connect.CodeNotFound, fmt.Errorf("monitor not found"))
		}
		return nil, s.failure(err)
	}
	return connect.NewResponse(m.ToProto()), nil
}
func (s *MonitorService) ListMonitors(ctx context.Context, r *connect.Request[monitorv1.ListMonitorsRequest]) (*connect.Response[monitorv1.ListMonitorsResponse], error) {
	page, size := int(r.Msg.Page), int(r.Msg.PageSize)
	if page < 1 {
		page = 1
	}
	if size < 1 {
		size = 50
	}
	if size > 200 {
		size = 200
	}
	f := &types.MonitorFilters{Page: page, PageSize: size, Enabled: r.Msg.Enabled}
	if r.Msg.Visibility != nil {
		v := string(models.VisibilityFromProto(*r.Msg.Visibility))
		f.Visibility = &v
	}
	ms, total, err := s.database.ListMonitors(ctx, f)
	if err != nil {
		return nil, s.failure(err)
	}
	out := &monitorv1.ListMonitorsResponse{Total: int32(total), Page: int32(page), PageSize: int32(size)}
	for _, m := range ms {
		out.Monitors = append(out.Monitors, m.ToProto())
	}
	return connect.NewResponse(out), nil
}
func (s *MonitorService) UpdateMonitor(ctx context.Context, r *connect.Request[monitorv1.UpdateMonitorRequest]) (*connect.Response[monitorv1.Monitor], error) {
	p := r.Msg
	m, err := s.database.GetMonitor(ctx, p.Id)
	if err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil, connect.NewError(connect.CodeNotFound, fmt.Errorf("monitor not found"))
		}
		return nil, s.failure(err)
	}
	u := &models.MonitorUpdate{Name: p.Name, URL: p.Url, IntervalSeconds: p.IntervalSeconds, TimeoutSeconds: p.TimeoutSeconds, Enabled: p.Enabled, Body: p.Body, Headers: p.Headers, ExpectedStatusCodes: p.ExpectedStatusCodes, PublicDomain: p.PublicDomain, PublicDisplayName: p.PublicDisplayName}
	if p.Name != nil {
		m.Name = *p.Name
	}
	if p.Url != nil {
		m.URL = *p.Url
	}
	if p.IntervalSeconds != nil {
		m.IntervalSeconds = *p.IntervalSeconds
	}
	if p.TimeoutSeconds != nil {
		m.TimeoutSeconds = *p.TimeoutSeconds
	}
	if p.Visibility != nil {
		if _, ok := monitorv1.MonitorVisibility_name[int32(*p.Visibility)]; !ok {
			return nil, connect.NewError(connect.CodeInvalidArgument, fmt.Errorf("invalid visibility"))
		}
		v := models.VisibilityFromProto(*p.Visibility)
		u.Visibility = &v
		m.Visibility = v
	}
	if p.Body != nil {
		m.Body = *p.Body
	}
	if len(p.Headers) > 0 {
		m.Headers = p.Headers
	}
	if len(p.ExpectedStatusCodes) > 0 {
		m.ExpectedStatusCodes = p.ExpectedStatusCodes
	}
	if err := validate(m); err != nil {
		return nil, connect.NewError(connect.CodeInvalidArgument, err)
	}
	if err := s.database.UpdateMonitor(ctx, p.Id, u); err != nil {
		return nil, s.failure(err)
	}
	return s.GetMonitor(ctx, connect.NewRequest(&monitorv1.GetMonitorRequest{Id: p.Id}))
}
func (s *MonitorService) DeleteMonitor(ctx context.Context, r *connect.Request[monitorv1.DeleteMonitorRequest]) (*connect.Response[emptypb.Empty], error) {
	if err := s.database.DeleteMonitor(ctx, r.Msg.Id); err != nil {
		return nil, s.failure(err)
	}
	return connect.NewResponse(&emptypb.Empty{}), nil
}
func (s *MonitorService) GetMonitorStats(ctx context.Context, r *connect.Request[monitorv1.GetMonitorStatsRequest]) (*connect.Response[monitorv1.MonitorStats], error) {
	p := r.Msg
	end := time.Unix(p.EndTime, 0)
	if p.EndTime == 0 {
		end = time.Now()
	}
	start := time.Unix(p.StartTime, 0)
	if p.StartTime == 0 {
		start = end.Add(-24 * time.Hour)
	}
	if start.After(end) || end.Sub(start) > 366*24*time.Hour {
		return nil, connect.NewError(connect.CodeInvalidArgument, fmt.Errorf("invalid time range"))
	}
	m, err := s.database.GetMonitorStats(ctx, p.MonitorId, start, end)
	if err != nil {
		return nil, s.failure(err)
	}
	return connect.NewResponse(&monitorv1.MonitorStats{MonitorId: m.MonitorID, TotalChecks: m.TotalChecks, SuccessfulChecks: m.SuccessfulChecks, FailedChecks: m.FailedChecks, UptimePercentage: m.UptimePercentage, AverageLatencyMs: m.AverageLatencyMs, MinLatencyMs: m.MinLatencyMs, MaxLatencyMs: m.MaxLatencyMs}), nil
}
func (s *MonitorService) failure(err error) error {
	s.logger.Error("Monitor operation failed", zap.Error(err))
	return connect.NewError(connect.CodeInternal, fmt.Errorf("monitor operation failed"))
}
