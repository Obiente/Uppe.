import { ResultStatus, type MonitoringResult } from '../../gen/result/v1/result_pb';
import type { Monitor } from '../../gen/monitor/v1/monitor_pb';
export function health(monitor: Monitor, result?: MonitoringResult) {
  if (!monitor.enabled) return 'paused';
  if (!result || Number(result.timestamp) <= Number(monitor.updatedAt) || Date.now()/1000 - Number(result.timestamp) > Math.max(monitor.intervalSeconds*2,monitor.timeoutSeconds+30)) return 'unknown';
  return result.status === ResultStatus.UP ? 'up' : result.status === ResultStatus.DEGRADED ? 'degraded' : 'down';
}
export const dateTime = (seconds: bigint | number) => new Date(Number(seconds)*1000).toLocaleString('en-GB',{dateStyle:'medium',timeStyle:'short',timeZone:'UTC'}) + ' UTC';
