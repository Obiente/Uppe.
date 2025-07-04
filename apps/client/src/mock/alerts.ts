export const mockAlerts = [
  {
    id: "1",
    type: "down" as const,
    monitor: "CDN",
    message: "Service is down - 15 peers confirm outage",
    timestamp: "2 hours ago",
    severity: "high" as const,
    acknowledged: false
  },
  {
    id: "2",
    type: "slow" as const,
    monitor: "Database",
    message: "Response time increased to 230ms (threshold: 200ms)",
    timestamp: "6 hours ago",
    severity: "medium" as const,
    acknowledged: false
  },
  {
    id: "3",
    type: "recovered" as const,
    monitor: "API Server",
    message: "Service recovered - all peers report normal operation",
    timestamp: "1 day ago",
    severity: "low" as const,
    acknowledged: true
  },
  {
    id: "4",
    type: "network" as const,
    monitor: "P2P Network",
    message: "New peer joined from Mumbai, IN",
    timestamp: "2 days ago",
    severity: "low" as const,
    acknowledged: true
  }
];
