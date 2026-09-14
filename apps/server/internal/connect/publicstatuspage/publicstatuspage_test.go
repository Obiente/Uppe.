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
	"github.com/Obiente/Uppe/apps/server/internal/models"
	"go.uber.org/zap"
	"google.golang.org/protobuf/encoding/protojson"
	"strings"
	"testing"
	"time"
)

type fixture struct {
	db.Database
	active                bool
	readError, errorStats bool
	pageError             error
	age                   time.Duration
}

func (f fixture) GetStatusPage(context.Context, string, bool) (*models.StatusPage, error) {
	return &models.StatusPage{ID: "page", Title: "Public services", Slug: "public-services", IsActive: f.active, MonitorIDs: []string{"one", "two"}}, f.pageError
}
func (f fixture) GetMonitor(context.Context, string) (*models.Monitor, error) {
	return &models.Monitor{Name: "Service", URL: "http://private-target.invalid", Body: "private-body", Headers: map[string]string{"Authorization": "private-token"}, Enabled: true, IntervalSeconds: 30, TimeoutSeconds: 10}, nil
}
func (f fixture) GetResults(context.Context, *types.ResultQuery) ([]*models.MonitoringResult, int, error) {
	if f.readError {
		return nil, 0, errors.New("synthetic read failure")
	}
	return []*models.MonitoringResult{{Status: resultv1.ResultStatus_RESULT_STATUS_UP, Timestamp: time.Now().Add(-f.age), MonitorNodeID: "private-peer"}}, 1, nil
}
func (f fixture) GetMonitorStats(context.Context, string, time.Time, time.Time) (*models.MonitorStats, error) {
	if f.errorStats {
		return nil, errors.New("synthetic stats failure")
	}
	return &models.MonitorStats{TotalChecks: 1, UptimePercentage: 100}, nil
}
func (f fixture) GetAggregatedStats(context.Context, string, time.Time, time.Time, types.AggregationPeriod) ([]*models.TimeSeriesDataPoint, error) {
	return nil, nil
}
func (f fixture) RecordStatusPageVisit(context.Context, string) error {
	return errors.New("counter unavailable")
}

func TestPublicProjection(t *testing.T) {
	for _, tt := range []struct {
		name    string
		fixture fixture
		code    connect.Code
		status  resultv1.ResultStatus
	}{
		{"healthy", fixture{active: true}, connect.CodeUnknown, resultv1.ResultStatus_RESULT_STATUS_UP},
		{"hidden", fixture{}, connect.CodeNotFound, 0},
		{"missing", fixture{pageError: sql.ErrNoRows}, connect.CodeNotFound, 0},
		{"database unavailable", fixture{pageError: errors.New("storage failure")}, connect.CodeUnavailable, 0},
		{"stale", fixture{active: true, age: 10 * time.Minute}, connect.CodeUnknown, 0},
		{"failed read", fixture{active: true, readError: true}, connect.CodeUnknown, 0},
		{"failed statistics", fixture{active: true, errorStats: true}, connect.CodeUnknown, 0},
	} {
		t.Run(tt.name, func(t *testing.T) {
			response, err := New(tt.fixture, zap.NewNop()).GetPublicStatusPage(context.Background(), connect.NewRequest(&pagev1.GetPublicStatusPageRequest{Slug: "public-services"}))
			if tt.code != connect.CodeUnknown {
				if connect.CodeOf(err) != tt.code {
					t.Fatalf("expected %v got %v", tt.code, err)
				}
				return
			}
			if err != nil {
				t.Fatal(err)
			}
			if len(response.Msg.Monitors) != 2 {
				t.Fatal("services disappeared after partial failure")
			}
			for _, m := range response.Msg.Monitors {
				if m.Status != tt.status {
					t.Fatalf("unexpected status: %v", m.Status)
				}
			}
			encoded, err := protojson.Marshal(response.Msg)
			if err != nil {
				t.Fatal(err)
			}
			if strings.Contains(string(encoded), "private-") {
				t.Fatal("private monitor data leaked")
			}
		})
	}
}
