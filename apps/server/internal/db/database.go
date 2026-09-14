package db

import (
	"github.com/Obiente/Uppe/apps/server/internal/config"
	"github.com/Obiente/Uppe/apps/server/internal/db/libsql"
	"time"
)

// Rust initializes the schema. This process never creates or migrates tables.
func NewDatabase(cfg *config.DatabaseConfig) (Database, error) {
	return libsql.NewWithRetry(cfg.Path, 15, time.Second)
}
