import { AggregationPeriod, ResultStatus, type TimeSeriesDataPoint } from '../../gen/result/v1/result_pb.ts';

export const ranges = { '24h': 86400, '7d': 604800, '30d': 2592000, '90d': 7776000 } as const;
export function historyRange(value: string | null, now = Math.floor(Date.now() / 1000)) {
  const key = value && Object.hasOwn(ranges, value) ? value as keyof typeof ranges : '24h';
  const step = key === '24h' ? 3600 : 86400;
  return { key, start: now - ranges[key], end: now, step, period: key === '24h' ? AggregationPeriod.HOUR : AggregationPeriod.DAY };
}

export interface HistoryBucket { timestamp: number; value: number | null; latency: number | null; checks: number }
export function historyBuckets(points: TimeSeriesDataPoint[], start: number, end: number, step: number): HistoryBucket[] {
  const byTime = new Map(points.map(p => [Math.floor(Number(p.timestamp) / step) * step, p]));
  const buckets: HistoryBucket[] = [];
  for (let time = Math.floor(start / step) * step; time <= end; time += step) {
    const point = byTime.get(time);
    const checks = Number(point?.totalChecks ?? 0);
    buckets.push({ timestamp: time, value: checks ? point!.uptimePercentage : null, latency: checks ? point!.averageLatencyMs : null, checks });
  }
  return buckets;
}
export function resultLabel(status: ResultStatus) {
  const labels: Partial<Record<ResultStatus,string>> = { [ResultStatus.UP]: 'Up', [ResultStatus.DOWN]: 'Down', [ResultStatus.DEGRADED]: 'Degraded', [ResultStatus.TIMEOUT]: 'Timeout', [ResultStatus.ERROR]: 'Error' };
  return labels[status] ?? 'Unknown';
}
export function badgeStatus(state: string) {
  return state === 'up' ? 'online' : state === 'down' ? 'offline' : state === 'paused' ? 'paused' : state === 'degraded' ? 'degraded' : 'unknown';
}
