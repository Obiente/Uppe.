package server

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

func TestAccessBoundary(t *testing.T) {
	token := strings.Repeat("synthetic-test-token-", 3)
	handler := accessControl(token, http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { w.WriteHeader(204) }))
	for _, test := range []struct {
		path, auth, origin string
		want               int
	}{
		{"/health", "", "", 204},
		{"/publicstatuspage.v1.PublicStatusPageService/GetPublicStatusPage", "", "", 204},
		{"/monitor.v1.MonitorService/ListMonitors", "", "", 401},
		{"/monitor.v1.MonitorService/DeleteMonitor", "Bearer wrong", "", 401},
		{"/monitor.v1.MonitorService/DeleteMonitor", "Bearer " + token, "", 204},
		{"/monitor.v1.MonitorService/DeleteMonitor", "Bearer " + token, "https://untrusted.example", 403},
	} {
		r := httptest.NewRequest("POST", test.path, nil)
		r.Header.Set("Authorization", test.auth)
		r.Header.Set("Origin", test.origin)
		w := httptest.NewRecorder()
		handler.ServeHTTP(w, r)
		if w.Code != test.want {
			t.Errorf("%s: got %d want %d", test.path, w.Code, test.want)
		}
	}
}
