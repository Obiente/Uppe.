package integration

import (
	"bytes"
	"context"
	"database/sql"
	"encoding/json"
	"fmt"
	"io"
	"net"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/Obiente/Uppe/apps/server/internal/config"
	"github.com/Obiente/Uppe/apps/server/internal/server"
	"go.uber.org/zap"
	_ "modernc.org/sqlite"
)

// Real processes exercise the shared schema, scheduler, signed gossip, and public
// projection together. All traffic stays on loopback and data lives in TempDir.
func TestRuntimeAndOpenPeerObservations(t *testing.T) {
	binary := os.Getenv("UPPE_TEST_SERVICE_BINARY")
	if binary == "" {
		t.Skip("set UPPE_TEST_SERVICE_BINARY to run the runtime contract")
	}
	port := func() int {
		l, err := net.Listen("tcp", "127.0.0.1:0")
		if err != nil {
			t.Fatal(err)
		}
		defer l.Close()
		return l.Addr().(*net.TCPAddr).Port
	}
	apiPort, peerPortA, peerPortB := port(), port(), port()
	startNode := func(peerPort int, bootstrap string) string {
		dir := t.TempDir()
		database := filepath.Join(dir, "uppe.db")
		cfg := filepath.Join(dir, "config.toml")
		bootstrapList := "[]"
		if bootstrap != "" {
			bootstrapList = fmt.Sprintf("[%q]", bootstrap)
		}
		text := fmt.Sprintf("[zeromq]\nbind='127.0.0.1'\nport=5555\n[preferences]\nuse_peerup_layer=true\nallow_peer_leech=false\naccept_remote_checks=false\nminimum_peer_mr=0\nlocation_privacy='disabled'\nenable_distributed_monitoring=false\nauto_sync_peer_results=false\n[peerup]\nport_range=[%d,%d]\nenable_mdns=false\nenable_kademlia=true\nenable_relay=false\nbootstrap_peers=%s\n", peerPort, peerPort, bootstrapList)
		if err := os.WriteFile(cfg, []byte(text), 0600); err != nil {
			t.Fatal(err)
		}
		env := append(os.Environ(), "UPPE_DATABASE_PATH="+database, "UPPE_DATA_DIR="+dir, "UPPE_KEYPAIR_PATH="+filepath.Join(dir, "node.key"))
		migrate := exec.Command(binary, "--config", cfg, "migrate")
		migrate.Env = env
		if out, err := migrate.CombinedOutput(); err != nil {
			t.Fatalf("migration: %v %s", err, out)
		}
		cmd := exec.Command(binary, "--config", cfg, "run")
		cmd.Env = env
		log, err := os.Create(filepath.Join(dir, "runtime.log"))
		if err != nil {
			t.Fatal(err)
		}
		cmd.Stdout = log
		cmd.Stderr = log
		if err := cmd.Start(); err != nil {
			t.Fatal(err)
		}
		t.Cleanup(func() {
			_ = cmd.Process.Kill()
			_ = cmd.Wait()
			_ = log.Close()
			if t.Failed() {
				data, _ := os.ReadFile(filepath.Join(dir, "runtime.log"))
				t.Log(string(data))
			}
		})
		return database
	}
	dbA := startNode(peerPortA, "")
	dbB := startNode(peerPortB, fmt.Sprintf("/ip4/127.0.0.1/tcp/%d", peerPortA))
	token := strings.Repeat("synthetic-integration-key-", 3)
	api, err := server.New(&config.Config{Address: fmt.Sprintf("127.0.0.1:%d", apiPort), Database: config.DatabaseConfig{Path: dbA}, OperatorToken: token}, zap.NewNop())
	if err != nil {
		t.Fatal(err)
	}
	go func() { _ = api.Start() }()
	t.Cleanup(func() {
		ctx, cancel := context.WithTimeout(context.Background(), time.Second)
		defer cancel()
		_ = api.Shutdown(ctx)
	})
	client := &http.Client{Timeout: 5 * time.Second}
	base := fmt.Sprintf("http://127.0.0.1:%d", apiPort)
	eventually := func(check func() bool) {
		t.Helper()
		deadline := time.Now().Add(60 * time.Second)
		for time.Now().Before(deadline) {
			if check() {
				return
			}
			time.Sleep(250 * time.Millisecond)
		}
		t.Fatal("runtime condition did not become true within 60 seconds")
	}
	eventually(func() bool {
		response, e := client.Get(base + "/health")
		if e != nil {
			return false
		}
		response.Body.Close()
		return response.StatusCode == 200
	})
	rpc := func(method string, body any, private bool) (int, map[string]any) {
		t.Helper()
		data, _ := json.Marshal(body)
		request, _ := http.NewRequest("POST", base+"/"+method, bytes.NewReader(data))
		request.Header.Set("Content-Type", "application/json")
		if private {
			request.Header.Set("Authorization", "Bearer "+token)
		}
		response, e := client.Do(request)
		if e != nil {
			t.Fatal(e)
		}
		defer response.Body.Close()
		raw, _ := io.ReadAll(response.Body)
		var result map[string]any
		_ = json.Unmarshal(raw, &result)
		return response.StatusCode, result
	}
	const monitors = "monitor.v1.MonitorService/"
	status, _ := rpc(monitors+"ListMonitors", map[string]any{}, false)
	if status != 401 {
		t.Fatalf("operator access without authentication: %d", status)
	}
	create := func(visibility string) string {
		t.Helper()
		status, result := rpc(monitors+"CreateMonitor", map[string]any{"name": "Synthetic " + visibility, "url": base + "/health", "type": "MONITOR_TYPE_HTTP", "intervalSeconds": 10, "timeoutSeconds": 2, "enabled": true, "visibility": visibility}, true)
		if status != 200 {
			t.Fatalf("create monitor: %d %v", status, result)
		}
		return result["id"].(string)
	}
	localID := create("MONITOR_VISIBILITY_INTERNAL")
	publicID := create("MONITOR_VISIBILITY_PUBLIC")
	readDB := func(path string) *sql.DB {
		d, e := sql.Open("sqlite", path)
		if e != nil {
			t.Fatal(e)
		}
		t.Cleanup(func() { d.Close() })
		return d
	}
	local, peer := readDB(dbA), readDB(dbB)
	eventually(func() bool {
		var count int
		_ = local.QueryRow("SELECT COUNT(*) FROM monitor_results WHERE monitor_uuid=? AND status='up'", localID).Scan(&count)
		return count > 0
	})
	eventually(func() bool {
		var count int
		_ = peer.QueryRow("SELECT COUNT(*) FROM peer_results WHERE monitor_uuid=? AND verified=1", publicID).Scan(&count)
		return count > 0
	})
	var count int
	if err = peer.QueryRow("SELECT COUNT(*) FROM monitor_results").Scan(&count); err != nil || count != 0 {
		t.Fatalf("remote observations influenced local health: %d %v", count, err)
	}
	if err = peer.QueryRow("SELECT COUNT(*) FROM peer_results WHERE monitor_uuid=?", localID).Scan(&count); err != nil || count != 0 {
		t.Fatalf("internal monitor was shared: %d %v", count, err)
	}
	// The intentionally non-public destination is blocked on the publishing node.
	if err = local.QueryRow("SELECT COUNT(*) FROM monitor_results WHERE monitor_uuid=? AND status='up'", publicID).Scan(&count); err != nil || count != 0 {
		t.Fatalf("public check reached a private address: %d %v", count, err)
	}
	const pages = "statuspage.v1.StatusPageService/"
	status, page := rpc(pages+"CreateStatusPage", map[string]any{"title": "Synthetic status", "slug": "synthetic-status", "monitorIds": []string{localID}}, true)
	if status != 200 {
		t.Fatalf("create status page: %d %v", status, page)
	}
	public := func() (int, map[string]any) {
		return rpc("publicstatuspage.v1.PublicStatusPageService/GetPublicStatusPage", map[string]any{"slug": "synthetic-status"}, false)
	}
	if status, _ = public(); status != 404 {
		t.Fatal("hidden page is public", status)
	}
	if status, _ = rpc(pages+"UpdateStatusPage", map[string]any{"id": page["id"], "isActive": true}, true); status != 200 {
		t.Fatal("publish", status)
	}
	status, result := public()
	encoded, _ := json.Marshal(result)
	if status != 200 || strings.Contains(string(encoded), base) || !strings.Contains(string(encoded), "RESULT_STATUS_UP") {
		t.Fatalf("unsafe or incorrect public projection: %d %s", status, encoded)
	}
	if status, _ = rpc(monitors+"UpdateMonitor", map[string]any{"id": localID, "enabled": false}, true); status != 200 {
		t.Fatal("pause", status)
	}
	_, result = public()
	encoded, _ = json.Marshal(result)
	if strings.Contains(string(encoded), "RESULT_STATUS_UP") {
		t.Fatal("paused monitor remains healthy")
	}
	if status, _ = rpc(pages+"UpdateStatusPage", map[string]any{"id": page["id"], "isActive": false}, true); status != 200 {
		t.Fatal("unpublish", status)
	}
	if status, _ = public(); status != 404 {
		t.Fatal("unpublish did not hide page", status)
	}
}
