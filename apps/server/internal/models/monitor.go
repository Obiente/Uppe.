package models

import (
	monitorv1 "github.com/Obiente/Uppe/apps/server/gen/monitor/v1"
	"time"
)

type MonitorVisibility string

const (
	MonitorVisibilityPublic   MonitorVisibility = "Public"
	MonitorVisibilityPrivate  MonitorVisibility = "Private"
	MonitorVisibilityInternal MonitorVisibility = "Internal"
)

type Monitor struct {
	ID, Name, URL                                        string
	Type                                                 monitorv1.MonitorType
	IntervalSeconds, TimeoutSeconds                      int32
	Enabled                                              bool
	ExpectedStatusCodes                                  []string
	Headers                                              map[string]string
	Body                                                 string
	UserID, OwnerPeerID, PublicDomain, PublicDisplayName *string
	Visibility                                           MonitorVisibility
	CreatedAt, UpdatedAt                                 time.Time
}

type MonitorUpdate struct {
	Name, URL, Body                 *string
	IntervalSeconds, TimeoutSeconds *int32
	Enabled                         *bool
	ExpectedStatusCodes             []string
	Headers                         map[string]string
	Visibility                      *MonitorVisibility
	PublicDomain, PublicDisplayName *string
}

type MonitorStats struct {
	MonitorID                                   string
	TotalChecks, SuccessfulChecks, FailedChecks int64
	UptimePercentage, AverageLatencyMs          float64
	MinLatencyMs, MaxLatencyMs                  int64
}

func (m *Monitor) ToProto() *monitorv1.Monitor {
	return &monitorv1.Monitor{Id: m.ID, Name: m.Name, Url: m.URL, Type: m.Type,
		IntervalSeconds: m.IntervalSeconds, TimeoutSeconds: m.TimeoutSeconds, Enabled: m.Enabled,
		ExpectedStatusCodes: m.ExpectedStatusCodes, Headers: m.Headers, Body: m.Body,
		Visibility: VisibilityToProto(m.Visibility), OwnerPeerId: value(m.OwnerPeerID), PublicDomain: value(m.PublicDomain),
		PublicDisplayName: value(m.PublicDisplayName), CreatedAt: m.CreatedAt.Unix(), UpdatedAt: m.UpdatedAt.Unix()}
}
func value(s *string) string {
	if s == nil {
		return ""
	}
	return *s
}
func VisibilityToProto(v MonitorVisibility) monitorv1.MonitorVisibility {
	switch v {
	case MonitorVisibilityPublic:
		return monitorv1.MonitorVisibility_MONITOR_VISIBILITY_PUBLIC
	case MonitorVisibilityPrivate:
		return monitorv1.MonitorVisibility_MONITOR_VISIBILITY_PRIVATE
	default:
		return monitorv1.MonitorVisibility_MONITOR_VISIBILITY_INTERNAL
	}
}
func VisibilityFromProto(v monitorv1.MonitorVisibility) MonitorVisibility {
	switch v {
	case monitorv1.MonitorVisibility_MONITOR_VISIBILITY_PUBLIC:
		return MonitorVisibilityPublic
	case monitorv1.MonitorVisibility_MONITOR_VISIBILITY_PRIVATE:
		return MonitorVisibilityPrivate
	default:
		return MonitorVisibilityInternal
	}
}
