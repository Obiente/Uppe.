package main

import (
	"context"
	"errors"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/Obiente/Uppe/apps/server/internal/config"
	"github.com/Obiente/Uppe/apps/server/internal/server"
	"go.uber.org/zap"
)

func main() {
	logger, _ := zap.NewProduction()
	defer logger.Sync()
	cfg, err := config.Load()
	if err != nil {
		logger.Error("Configuration failed", zap.Error(err))
		os.Exit(1)
	}
	app, err := server.New(cfg, logger)
	if err != nil {
		logger.Error("Startup failed", zap.Error(err))
		os.Exit(1)
	}
	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()
	done := make(chan error, 1)
	go func() { done <- app.Start() }()
	select {
	case err := <-done:
		if !errors.Is(err, http.ErrServerClosed) {
			logger.Error("Server failed", zap.Error(err))
			os.Exit(1)
		}
	case <-ctx.Done():
		deadline, cancel := context.WithTimeout(context.Background(), 10*time.Second)
		defer cancel()
		if err := app.Shutdown(deadline); err != nil {
			logger.Error("Shutdown failed", zap.Error(err))
			os.Exit(1)
		}
	}
}
