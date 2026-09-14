package libsql

import (
	"context"
	"os"
	"os/exec"
	"path/filepath"
	"testing"
	"time"

	monitorv1 "github.com/Obiente/Uppe/apps/server/gen/monitor/v1"
	"github.com/Obiente/Uppe/apps/server/internal/db/types"
	"github.com/Obiente/Uppe/apps/server/internal/models"
)

// Exercise the actual Rust schema, including triggers, without maintaining a
// second schema in Go. CI and scripts/check.mjs provide the service executable.
func TestRustSchemaContract(t *testing.T) {
	binary := os.Getenv("UPPE_TEST_SERVICE_BINARY")
	if binary == "" {
		t.Skip("set UPPE_TEST_SERVICE_BINARY to run the Rust/Go integration contract")
	}
	dir := t.TempDir()
	dbPath := filepath.Join(dir, "contract.db")
	cmd := exec.Command(binary, "--config", filepath.Join(dir, "config.toml"), "migrate")
	cmd.Env = append(os.Environ(), "UPPE_DATABASE_PATH="+dbPath)
	if output, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("Rust migration: %v\n%s", err, output)
	}
	d, err := New(dbPath)
	if err != nil {
		t.Fatal(err)
	}
	defer d.Close()
	ctx := context.Background()
	m := &models.Monitor{Name: "Synthetic TCP", URL: "127.0.0.1:43210", Type: monitorv1.MonitorType_MONITOR_TYPE_TCP, IntervalSeconds: 10, TimeoutSeconds: 2, Enabled: true, Visibility: models.MonitorVisibilityInternal}
	if err = d.CreateMonitor(ctx, m); err != nil {
		t.Fatal(err)
	}
	loaded, err := d.GetMonitor(ctx, m.ID)
	if err != nil {
		t.Fatal(err)
	}
	if loaded.Type != m.Type || loaded.Visibility != m.Visibility {
		t.Fatalf("wrong round trip: %+v", loaded)
	}
	var kind string
	if err = d.db.QueryRow("SELECT check_type FROM monitors WHERE uuid=?", m.ID).Scan(&kind); err != nil || kind != "tcp" {
		t.Fatalf("Rust check type: %s %v", kind, err)
	}
	target := "127.0.0.1:43211"
	if err = d.UpdateMonitor(ctx, m.ID, &models.MonitorUpdate{URL: &target}); err != nil {
		t.Fatal(err)
	}
	loaded, err = d.GetMonitor(ctx, m.ID)
	if err != nil || loaded.URL != target {
		t.Fatal("target-only update did not persist", err)
	}
	stats, err := d.GetMonitorStats(ctx, m.ID, time.Now().Add(-time.Hour), time.Now())
	if err != nil || stats.TotalChecks != 0 {
		t.Fatal("empty statistics", err)
	}
	if _, _, err = d.GetResults(ctx, &types.ResultQuery{MonitorID: m.ID, StartTime: time.Now().Add(-time.Hour), EndTime: time.Now(), Limit: 1}); err != nil {
		t.Fatal(err)
	}
	if _, err = d.GetAggregatedStats(ctx, m.ID, time.Now().Add(-time.Hour), time.Now(), types.AggregationPeriodHour); err != nil {
		t.Fatal(err)
	}
	p := &models.StatusPage{Title: "Synthetic status", Slug: "synthetic-status", MonitorIDs: []string{m.ID}, IsActive: false}
	if err = d.CreateStatusPage(ctx, p); err != nil {
		t.Fatal(err)
	}
	page, err := d.GetStatusPage(ctx, p.Slug, true)
	if err != nil {
		t.Fatal(err)
	}
	if page.IsActive || len(page.MonitorIDs) != 1 {
		t.Fatal("incorrect publication")
	}
	// A nested hydration query must not hold the pool's only connection.
	d.db.SetMaxOpenConns(1)
	listCtx, cancel := context.WithTimeout(ctx, 2*time.Second)
	defer cancel()
	pages, total, err := d.ListStatusPages(listCtx, 1, 25, false)
	if err != nil || total != 1 || len(pages) != 1 || len(pages[0].MonitorIDs) != 1 {
		t.Fatal("status list connection deadlock or lost binding", err)
	}
	active := true
	if err = d.UpdateStatusPage(ctx, p.ID, &models.StatusPageUpdate{IsActive: &active}); err != nil {
		t.Fatal(err)
	}
	// Failed replacement rolls back the removal and the page metadata together.
	title := "Should roll back"
	if err = d.UpdateStatusPage(ctx, p.ID, &models.StatusPageUpdate{Title: &title, ReplaceMonitorIDs: true, MonitorIDs: []string{"missing-monitor"}}); err == nil {
		t.Fatal("invalid binding accepted")
	}
	page, err = d.GetStatusPage(ctx, p.ID, false)
	if err != nil || page.Title != p.Title || len(page.MonitorIDs) != 1 {
		t.Fatal("partial transaction", err)
	}
	if err = d.UpdateStatusPage(ctx, p.ID, &models.StatusPageUpdate{ReplaceMonitorIDs: true, MonitorIDs: []string{}}); err != nil {
		t.Fatal(err)
	}
	page, err = d.GetStatusPage(ctx, p.ID, false)
	if err != nil || len(page.MonitorIDs) != 0 {
		t.Fatal("empty replacement ignored", err)
	}
	var before, after int
	d.db.QueryRow("SELECT COUNT(*) FROM audit_outbox").Scan(&before)
	if err = d.RecordStatusPageVisit(ctx, p.ID); err != nil {
		t.Fatal(err)
	}
	d.db.QueryRow("SELECT COUNT(*) FROM audit_outbox").Scan(&after)
	if before == 0 || before != after {
		t.Fatal("visit should not create audit events", before, after)
	}
	if err = d.DeleteStatusPage(ctx, p.ID); err != nil {
		t.Fatal(err)
	}
	if err = d.DeleteMonitor(ctx, m.ID); err != nil {
		t.Fatal(err)
	}
}
