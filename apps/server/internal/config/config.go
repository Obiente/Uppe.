package config

import (
	"fmt"
	"net"
	"os"
	"path/filepath"
)

type DatabaseConfig struct{ Path string }
type Config struct {
	Address       string
	Database      DatabaseConfig
	OperatorToken string
}

func Load() (*Config, error) {
	dataDir := os.Getenv("UPPE_DATA_DIR")
	if dataDir == "" {
		dataDir = ".uppe"
	}
	dbPath := os.Getenv("UPPE_DATABASE_PATH")
	if dbPath == "" {
		dbPath = filepath.Join(dataDir, "uppe.db")
	}
	address := os.Getenv("UPPE_API_ADDRESS")
	if address == "" {
		address = "127.0.0.1:8080"
	}
	if _, _, err := net.SplitHostPort(address); err != nil {
		return nil, fmt.Errorf("invalid UPPE_API_ADDRESS: %w", err)
	}
	token := os.Getenv("UPPE_OPERATOR_TOKEN")
	if len(token) < 32 {
		return nil, fmt.Errorf("UPPE_OPERATOR_TOKEN must contain at least 32 characters")
	}
	return &Config{Address: address, Database: DatabaseConfig{Path: dbPath}, OperatorToken: token}, nil
}
func (c *Config) ServerAddress() string { return c.Address }
