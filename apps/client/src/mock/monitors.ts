export const mockMonitors = [
  {
    title: "Main Website",
    url: "https://myapp.com",
    status: "online" as const,
    uptime: 99.9,
    responseTime: 145,
    lastCheck: "2 min ago",
    region: "Global"
  },
  {
    title: "API Server",
    url: "https://api.myapp.com",
    status: "online" as const,
    uptime: 99.8,
    responseTime: 89,
    lastCheck: "1 min ago",
    region: "Global"
  },
  {
    title: "Database",
    url: "https://db.myapp.com",
    status: "warning" as const,
    uptime: 98.5,
    responseTime: 230,
    lastCheck: "3 min ago",
    region: "Global"
  },
  {
    title: "CDN",
    url: "https://cdn.myapp.com",
    status: "offline" as const,
    uptime: 95.2,
    responseTime: 0,
    lastCheck: "5 min ago",
    region: "Global"
  }
];

export const mockDetailedMonitors = [
  {
    id: "1",
    name: "Main Website",
    url: "https://myapp.com",
    status: "online" as const,
    uptime: 99.9,
    avgPing: 145,
    totalChecks: 15420,
    lastIncident: "2 days ago",
    peerCoverage: 23
  },
  {
    id: "2",
    name: "API Server",
    url: "https://api.myapp.com",
    status: "online" as const,
    uptime: 99.8,
    avgPing: 89,
    totalChecks: 18230,
    lastIncident: null,
    peerCoverage: 31
  },
  {
    id: "3",
    name: "Database",
    url: "https://db.myapp.com",
    status: "warning" as const,
    uptime: 98.5,
    avgPing: 230,
    totalChecks: 12100,
    lastIncident: "6 hours ago",
    peerCoverage: 18
  },
  {
    id: "4",
    name: "CDN",
    url: "https://cdn.myapp.com",
    status: "offline" as const,
    uptime: 95.2,
    avgPing: 0,
    totalChecks: 9850,
    lastIncident: "2 hours ago",
    peerCoverage: 15
  }
];
