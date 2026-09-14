package server

import (
	"net/http"
	"sync"
	"time"
)

// A global admission budget works behind a proxy without trusting spoofable
// forwarded addresses. Operator routes have a separate budget (authentication).
func limitPublic(next http.Handler) http.Handler {
	var mu sync.Mutex
	tokens := 60.0
	last := time.Now()
	active := make(chan struct{}, 16)
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		mu.Lock()
		now := time.Now()
		tokens = min(60, tokens+now.Sub(last).Seconds()*30)
		last = now
		allowed := tokens >= 1
		if allowed {
			tokens--
		}
		mu.Unlock()
		if !allowed {
			w.Header().Set("Retry-After", "1")
			http.Error(w, "Public request limit reached", http.StatusTooManyRequests)
			return
		}
		select {
		case active <- struct{}{}:
			defer func() { <-active }()
		default:
			w.Header().Set("Retry-After", "1")
			http.Error(w, "Public request capacity reached", http.StatusTooManyRequests)
			return
		}
		next.ServeHTTP(w, r)
	})
}
