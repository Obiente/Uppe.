package models

import "time"

// StatusPage represents a public status page persisted in the shared database.
type StatusPage struct {
	ID           string
	Title        string
	Slug         string
	Description  string
	CustomDomain *string
	MonitorIDs   []string
	IsActive     bool
	LogoURL      *string
	PrimaryColor string
	Uptime       float64
	Visits       int64
	LastIncident *time.Time
	CreatedAt    time.Time
	UpdatedAt    time.Time
}

// StatusPageUpdate represents mutable status page fields.
type StatusPageUpdate struct {
	Title             *string
	Slug              *string
	Description       *string
	MonitorIDs        []string
	ReplaceMonitorIDs bool
	IsActive          *bool
	LogoURL           *string
	PrimaryColor      *string
}
