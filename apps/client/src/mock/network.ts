export const mockGlobalPingData = [
  {
    id: "peer_1a2b3c",
    location: "New York, US",
    latitude: 40.7128,
    longitude: -74.0060,
    ping: 45,
    status: "online" as const,
    lastCheck: "1 min ago"
  },
  {
    id: "peer_4d5e6f",
    location: "London, UK",
    latitude: 51.5074,
    longitude: -0.1278,
    ping: 78,
    status: "online" as const,
    lastCheck: "2 min ago"
  },
  {
    id: "peer_7g8h9i",
    location: "Tokyo, JP",
    latitude: 35.6762,
    longitude: 139.6503,
    ping: 120,
    status: "online" as const,
    lastCheck: "1 min ago"
  },
  {
    id: "peer_0j1k2l",
    location: "Sydney, AU",
    latitude: -33.8688,
    longitude: 151.2093,
    ping: 180,
    status: "timeout" as const,
    lastCheck: "5 min ago"
  },
  {
    id: "peer_3m4n5o",
    location: "São Paulo, BR",
    latitude: -23.5505,
    longitude: -46.6333,
    ping: 95,
    status: "online" as const,
    lastCheck: "3 min ago"
  },
  {
    id: "peer_6p7q8r",
    location: "Mumbai, IN",
    latitude: 19.0760,
    longitude: 72.8777,
    ping: 0,
    status: "error" as const,
    lastCheck: "10 min ago"
  }
];
