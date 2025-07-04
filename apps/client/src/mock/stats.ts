// Uptime data for charts - 24 hours of data
export const mockUptimeData = Array.from({ length: 24 }, (_, i) => {
  const hour = i;
  if (hour < 2) return 95.2; // CDN issues
  if (hour < 6) return 98.5; // Database issues
  return 99.9; // Normal operation
});

export const mockHours = Array.from({ length: 24 }, (_, i) => i);

// P2P Network Stats
export const mockNetworkStats = {
  shareRatio: 0.85, // 85% balance
  monitoringForOthers: 4250,
  receivingFromOthers: 3680,
  contributionScore: 92,
  connectedPeers: 147,
  totalPeers: 1823,
  networkHealth: 96,
  myNodeId: "peer_a1b2c3d4e5f6",
  bandwidthUsed: 78,
  bandwidthLimit: 100,
  checksPerformedToday: 2840,
  checksReceivedToday: 3120
};
