package publicstatuspage

import (
	"connectrpc.com/connect"
	"context"
	pagev1 "github.com/Obiente/Uppe/apps/server/gen/publicstatuspage/v1"
	"github.com/Obiente/Uppe/apps/server/internal/models"
	"go.uber.org/zap"
	"sync"
	"sync/atomic"
	"testing"
	"time"
)

type countedFixture struct {
	fixture
	reads   atomic.Int64
	visits  atomic.Int64
	hidden  atomic.Bool
	started chan struct{}
	release chan struct{}
	once    sync.Once
}

func (f *countedFixture) GetMonitor(ctx context.Context, id string) (*models.Monitor, error) {
	f.reads.Add(1)
	if f.started != nil {
		f.once.Do(func() { close(f.started) })
		select {
		case <-f.release:
		case <-ctx.Done():
			return nil, ctx.Err()
		}
	}
	return f.fixture.GetMonitor(ctx, id)
}
func (f *countedFixture) GetStatusPage(ctx context.Context, id string, slug bool) (*models.StatusPage, error) {
	p, e := f.fixture.GetStatusPage(ctx, id, slug)
	p.IsActive = !f.hidden.Load()
	return p, e
}
func (f *countedFixture) AddStatusPageVisits(_ context.Context, _ string, count int64) error {
	f.visits.Add(count)
	return nil
}

func TestCacheCoalescesAndRevokes(t *testing.T) {
	f := &countedFixture{fixture: fixture{active: true}, started: make(chan struct{}), release: make(chan struct{})}
	s := New(f, zap.NewNop())
	ctx := context.Background()
	read := func() (*connect.Response[pagev1.PublicStatusPage], error) {
		return s.GetPublicStatusPage(ctx, connect.NewRequest(&pagev1.GetPublicStatusPageRequest{Slug: "public-services"}))
	}
	first := make(chan error, 1)
	go func() { _, err := read(); first <- err }()
	<-f.started
	second := make(chan error, 1)
	go func() { _, err := read(); second <- err }()
	close(f.release)
	if err := <-first; err != nil {
		t.Fatal(err)
	}
	if err := <-second; err != nil {
		t.Fatal(err)
	}
	if f.reads.Load() != 2 {
		t.Fatalf("expected one projection for two monitors, got %d reads", f.reads.Load())
	}
	page, err := read()
	if err != nil {
		t.Fatal(err)
	}
	page.Msg.Title = "caller mutation"
	other, _ := read()
	if other.Msg.Title == "caller mutation" {
		t.Fatal("cache exposed mutable shared response")
	}
	if f.visits.Load() != 0 {
		t.Fatal("a public read synchronously wrote analytics")
	}
	s.flushVisits(ctx)
	if f.visits.Load() != 4 {
		t.Fatalf("batched visits: %d", f.visits.Load())
	}
	f.hidden.Store(true)
	if _, err := read(); connect.CodeOf(err) != connect.CodeNotFound {
		t.Fatal("unpublished page escaped cache", err)
	}
	f.hidden.Store(false)
	s.Invalidate()
	if _, err := read(); err != nil {
		t.Fatal(err)
	}
	if f.reads.Load() != 4 {
		t.Fatal("mutation did not invalidate projection")
	}
	// Expire the actual fixture key, independent of its display metadata.
	s.cache.mu.Lock()
	for key, entry := range s.cache.entries {
		entry.until = time.Now().Add(-time.Second)
		s.cache.entries[key] = entry
	}
	s.cache.mu.Unlock()
	if _, err := read(); err != nil {
		t.Fatal(err)
	}
	if f.reads.Load() != 6 {
		t.Fatal("expired cache did not refresh")
	}
}

func TestInvalidationDuringRefreshDoesNotServePreviousGeneration(t *testing.T) {
	f := &countedFixture{fixture: fixture{active: true}, started: make(chan struct{}), release: make(chan struct{})}
	s := New(f, zap.NewNop())
	read := func() error {
		_, err := s.GetPublicStatusPage(context.Background(), connect.NewRequest(&pagev1.GetPublicStatusPageRequest{Slug: "public-services"}))
		return err
	}
	done := make(chan error, 1)
	go func() { done <- read() }()
	<-f.started
	s.Invalidate()
	err := read()
	close(f.release)
	if connect.CodeOf(err) != connect.CodeUnavailable {
		t.Fatal("request joined a superseded refresh", err)
	}
	if err = <-done; err != nil {
		t.Fatal(err)
	}
	s.cache.mu.Lock()
	cached := len(s.cache.entries)
	s.cache.mu.Unlock()
	if cached != 0 {
		t.Fatal("superseded refresh repopulated cache")
	}
	if err = read(); err != nil {
		t.Fatal(err)
	}
	if f.reads.Load() != 4 {
		t.Fatal("fresh generation was not loaded")
	}
}
