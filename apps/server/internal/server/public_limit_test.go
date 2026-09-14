package server

import (
	"net/http"
	"net/http/httptest"
	"sync"
	"testing"
)

func TestPublicRateLimit(t *testing.T) {
	calls := 0
	handler := limitPublic(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { calls++; w.WriteHeader(204) }))
	rejected := 0
	for i := 0; i < 100; i++ {
		w := httptest.NewRecorder()
		handler.ServeHTTP(w, httptest.NewRequest("POST", "/public", nil))
		if w.Code == 429 {
			rejected++
		}
	}
	if rejected == 0 || calls > 65 {
		t.Fatalf("public work was not bounded: %d calls, %d rejected", calls, rejected)
	}
}

func TestPublicConcurrentCapacityRecovers(t *testing.T) {
	started := make(chan struct{}, 16)
	release := make(chan struct{})
	var running sync.WaitGroup
	handler := limitPublic(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { started <- struct{}{}; <-release; w.WriteHeader(204) }))
	for i := 0; i < 16; i++ {
		running.Add(1)
		go func() {
			defer running.Done()
			handler.ServeHTTP(httptest.NewRecorder(), httptest.NewRequest("POST", "/public", nil))
		}()
	}
	for i := 0; i < 16; i++ {
		<-started
	}
	response := httptest.NewRecorder()
	handler.ServeHTTP(response, httptest.NewRequest("POST", "/public", nil))
	if response.Code != 429 {
		t.Errorf("excess concurrent request was admitted: %d", response.Code)
	}
	close(release)
	running.Wait()
	response = httptest.NewRecorder()
	handler.ServeHTTP(response, httptest.NewRequest("POST", "/public", nil))
	if response.Code != 204 {
		t.Fatal("capacity did not recover")
	}
}
