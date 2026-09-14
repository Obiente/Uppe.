package server

import (
	"context"
	publicconnect "github.com/Obiente/Uppe/apps/server/gen/publicstatuspage/v1/publicstatuspagev1connect"
	publicservice "github.com/Obiente/Uppe/apps/server/internal/connect/publicstatuspage"
	"net/http"
	"time"

	"go.uber.org/zap"

	monitorv1connect "github.com/Obiente/Uppe/apps/server/gen/monitor/v1/monitorv1connect"
	networkv1connect "github.com/Obiente/Uppe/apps/server/gen/network/v1/networkv1connect"
	resultv1connect "github.com/Obiente/Uppe/apps/server/gen/result/v1/resultv1connect"
	settingsv1connect "github.com/Obiente/Uppe/apps/server/gen/settings/v1/settingsv1connect"
	statuspagev1connect "github.com/Obiente/Uppe/apps/server/gen/statuspage/v1/statuspagev1connect"
	"github.com/Obiente/Uppe/apps/server/internal/config"
	monitorservice "github.com/Obiente/Uppe/apps/server/internal/connect/monitor"
	networkservice "github.com/Obiente/Uppe/apps/server/internal/connect/network"
	resultservice "github.com/Obiente/Uppe/apps/server/internal/connect/result"
	settingsservice "github.com/Obiente/Uppe/apps/server/internal/connect/settings"
	statuspageservice "github.com/Obiente/Uppe/apps/server/internal/connect/statuspage"
	"github.com/Obiente/Uppe/apps/server/internal/db"
)

type Server struct {
	httpServer *http.Server
	logger     *zap.Logger
	database   db.Database
}

// Create an authenticated API with a separate public status projection.
func New(cfg *config.Config, logger *zap.Logger) (*Server, error) {
	// Initialize database
	database, err := db.NewDatabase(&cfg.Database)
	if err != nil {
		return nil, err
	}

	// Test database connection
	if err := database.Ping(context.Background()); err != nil {
		database.Close()
		return nil, err
	}

	// Create HTTP mux
	mux := http.NewServeMux()

	// Create service handlers with database
	monitorService := monitorservice.NewMonitorService(logger, database)
	networkService := networkservice.NewNetworkService(logger, database)
	resultService := resultservice.NewResultService(logger, database)
	settingsService := settingsservice.NewSettingsService(logger, database)
	statusPageService := statuspageservice.NewStatusPageService(logger, database)

	// Register ConnectRPC handlers
	monitorPath, monitorHandler := monitorv1connect.NewMonitorServiceHandler(monitorService)
	networkPath, networkHandler := networkv1connect.NewNetworkServiceHandler(networkService)
	resultPath, resultHandler := resultv1connect.NewResultServiceHandler(resultService)
	settingsPath, settingsHandler := settingsv1connect.NewSettingsServiceHandler(settingsService)
	statusPagePath, statusPageHandler := statuspagev1connect.NewStatusPageServiceHandler(statusPageService)

	mux.Handle(monitorPath, monitorHandler)
	mux.Handle(networkPath, networkHandler)
	mux.Handle(resultPath, resultHandler)
	mux.Handle(settingsPath, settingsHandler)
	mux.Handle(statusPagePath, statusPageHandler)
	publicPath, publicHandler := publicconnect.NewPublicStatusPageServiceHandler(publicservice.New(database, logger))
	mux.Handle(publicPath, publicHandler)

	// Health check endpoint
	mux.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
		w.Write([]byte("OK"))
	})

	// Connect uses HTTP/1.1 behind the Astro proxy. Do not expose an unauthenticated h2c upgrade.
	httpServer := &http.Server{
		Addr:              cfg.ServerAddress(),
		Handler:           accessControl(cfg.OperatorToken, mux),
		ReadHeaderTimeout: 5 * time.Second,
		MaxHeaderBytes:    16 << 10,
		ReadTimeout:       15 * time.Second,
		WriteTimeout:      15 * time.Second,
		IdleTimeout:       60 * time.Second,
	}

	return &Server{
		httpServer: httpServer,
		logger:     logger,
		database:   database,
	}, nil
}

func (s *Server) Start() error {
	s.logger.Info("Server starting",
		zap.String("address", s.httpServer.Addr),
	)
	return s.httpServer.ListenAndServe()
}

func (s *Server) Shutdown(ctx context.Context) error {
	if err := s.httpServer.Shutdown(ctx); err != nil {
		return err
	}
	return s.database.Close()
}
