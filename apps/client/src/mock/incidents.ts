export const mockIncidents = [
  {
    id: "1",
    title: "CDN Performance Issues",
    status: "ongoing" as const,
    time: "2 hours ago",
    duration: "2h 15m",
    description: "Multiple peers reporting slow response times"
  },
  {
    id: "2",
    title: "Database Connection Timeout",
    status: "resolved" as const,
    time: "6 hours ago",
    duration: "45m",
    description: "Brief database connectivity issues resolved"
  }
];
