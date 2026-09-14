package server

import (
	"crypto/sha256"
	"crypto/subtle"
	"net/http"
	"strings"
)

// Only the dedicated public projection and process health are unauthenticated.
// Operator calls use a server-side token. The browser receives an expiring
// HttpOnly session from the Astro application, never this upstream credential.
func accessControl(token string, next http.Handler) http.Handler {
	expected := sha256.Sum256([]byte(token))
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("X-Content-Type-Options", "nosniff")
		w.Header().Set("Cache-Control", "no-store")
		r.Body = http.MaxBytesReader(w, r.Body, 1<<20)
		if r.URL.Path != "/health" && r.URL.Path != "/publicstatuspage.v1.PublicStatusPageService/GetPublicStatusPage" {
			value, ok := strings.CutPrefix(r.Header.Get("Authorization"), "Bearer ")
			actual := sha256.Sum256([]byte(value))
			if !ok || len(token) < 32 || subtle.ConstantTimeCompare(actual[:], expected[:]) != 1 {
				w.Header().Set("Content-Type", "application/json")
				w.WriteHeader(http.StatusUnauthorized)
				w.Write([]byte(`{"code":"unauthenticated","message":"Sign in to continue"}`))
				return
			}
		}
		// No cross-origin credentials: browser traffic goes through the same-origin application.
		if r.Header.Get("Origin") != "" {
			http.Error(w, "Cross-origin access is disabled", http.StatusForbidden)
			return
		}
		next.ServeHTTP(w, r)
	})
}
