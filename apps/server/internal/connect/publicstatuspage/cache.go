package publicstatuspage

import (
	"connectrpc.com/connect"
	"context"
	"errors"
	pagev1 "github.com/Obiente/Uppe/apps/server/gen/publicstatuspage/v1"
	"github.com/Obiente/Uppe/apps/server/internal/models"
	"google.golang.org/protobuf/proto"
	"sync"
	"time"
)

const cacheTTL = 5 * time.Second

type cachedPage struct {
	page  *pagev1.PublicStatusPage
	until time.Time
}
type pageFlight struct {
	generation uint64
	done       chan struct{}
	page       *pagev1.PublicStatusPage
	err        error
}
type pageCache struct {
	mu         sync.Mutex
	entries    map[string]cachedPage
	flights    map[string]*pageFlight
	visits     map[string]int64
	generation uint64
	refresh    chan struct{}
}

func newCache() *pageCache {
	return &pageCache{entries: make(map[string]cachedPage), flights: make(map[string]*pageFlight), visits: make(map[string]int64), refresh: make(chan struct{}, 1)}
}
func (s *Service) Invalidate() {
	s.cache.mu.Lock()
	defer s.cache.mu.Unlock()
	s.cache.generation++
	clear(s.cache.entries)
}

func (s *Service) cached(ctx context.Context, page *models.StatusPage) (*connect.Response[pagev1.PublicStatusPage], error) {
	c := s.cache
	now := time.Now()
	c.mu.Lock()
	if len(c.visits) < 128 || c.visits[page.ID] > 0 {
		c.visits[page.ID]++
	}
	if entry, ok := c.entries[page.ID]; ok && now.Before(entry.until) {
		result := proto.Clone(entry.page).(*pagev1.PublicStatusPage)
		c.mu.Unlock()
		return connect.NewResponse(result), nil
	}
	if flight, ok := c.flights[page.ID]; ok {
		if flight.generation != c.generation {
			c.mu.Unlock()
			return nil, connect.NewError(connect.CodeUnavailable, errors.New("page changed during refresh"))
		}
		c.mu.Unlock()
		select {
		case <-ctx.Done():
			return nil, connect.NewError(connect.CodeCanceled, ctx.Err())
		case <-flight.done:
		}
		if flight.err != nil {
			return nil, flight.err
		}
		return connect.NewResponse(proto.Clone(flight.page).(*pagev1.PublicStatusPage)), nil
	}
	// Only one public refresh may use the shared database at a time. Different
	// pages fail fast instead of queueing work ahead of management requests.
	select {
	case c.refresh <- struct{}{}:
	default:
		c.mu.Unlock()
		return nil, connect.NewError(connect.CodeResourceExhausted, errors.New("public page refresh capacity reached"))
	}
	flight := &pageFlight{done: make(chan struct{}), generation: c.generation}
	c.flights[page.ID] = flight
	generation := c.generation
	c.mu.Unlock()
	refreshCtx, cancel := context.WithTimeout(ctx, 2*time.Second)
	response, err := s.build(refreshCtx, page)
	cancel()
	c.mu.Lock()
	if err == nil {
		flight.page = response.Msg
		if generation == c.generation {
			for key, entry := range c.entries {
				if !now.Before(entry.until) {
					delete(c.entries, key)
				}
			}
			if len(c.entries) >= 128 {
				for key := range c.entries {
					delete(c.entries, key)
					break
				}
			}
			c.entries[page.ID] = cachedPage{response.Msg, time.Now().Add(cacheTTL)}
		}
	}
	flight.err = err
	delete(c.flights, page.ID)
	close(flight.done)
	<-c.refresh
	c.mu.Unlock()
	if err != nil {
		return nil, err
	}
	return connect.NewResponse(proto.Clone(response.Msg).(*pagev1.PublicStatusPage)), nil
}

// RunVisits aggregates best-effort analytics without adding a write to public reads.
// The server owns its lifetime and cancels it during shutdown.
func (s *Service) RunVisits(ctx context.Context) {
	ticker := time.NewTicker(30 * time.Second)
	defer ticker.Stop()
	for {
		select {
		case <-ctx.Done():
			return
		case <-ticker.C:
			s.flushVisits(ctx)
		}
	}
}
func (s *Service) flushVisits(ctx context.Context) {
	c := s.cache
	c.mu.Lock()
	visits := c.visits
	c.visits = make(map[string]int64)
	c.mu.Unlock()
	for id, count := range visits {
		if ctx.Err() != nil {
			return
		}
		writeCtx, cancel := context.WithTimeout(ctx, time.Second)
		_ = s.database.AddStatusPageVisits(writeCtx, id, count)
		cancel()
	}
}
