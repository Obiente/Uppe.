import assert from 'node:assert/strict';
import { test } from 'node:test';
import { create } from '@bufbuild/protobuf';
import { MonitorSchema } from '../../gen/monitor/v1/monitor_pb.ts';
import { MonitoringResultSchema, TimeSeriesDataPointSchema, ResultStatus } from '../../gen/result/v1/result_pb.ts';
import { historyRange, historyBuckets } from './history.ts';
import { health } from './health.ts';

test('missing periods stay empty and calendar-aligned observations land in the correct bucket', () => {
  const points=[create(TimeSeriesDataPointSchema,{timestamp:3700n,totalChecks:2n,successfulChecks:1n,uptimePercentage:50,averageLatencyMs:250})];
  const buckets=historyBuckets(points,120,7300,3600);
  assert.deepEqual(buckets.map(b=>b.value),[null,50,null]);
  assert.deepEqual(buckets.map(b=>b.latency),[null,250,null]);
  assert.deepEqual(historyBuckets([],0,3600,3600).map(b=>b.value),[null,null]);
});
test('time ranges are bounded and invalid input falls back to 24 hours',()=>{
  assert.equal(historyRange('forever',100000).key,'24h');
  assert.equal(historyRange('90d',10000000).start,2224000);
  assert.equal(historyRange('7d',1000000).step,86400);
});
test('unknown, stale and paused checks never become outages or healthy readings',()=>{
  const now=BigInt(Math.floor(Date.now()/1000));
  const monitor=create(MonitorSchema,{enabled:true,intervalSeconds:30,timeoutSeconds:10,updatedAt:now-300n});
  const result=create(MonitoringResultSchema,{timestamp:now,status:ResultStatus.UNSPECIFIED});
  assert.equal(health(monitor,result),'unknown');
  result.status=99 as ResultStatus;assert.equal(health(monitor,result),'unknown');
  result.status=ResultStatus.DOWN;assert.equal(health(monitor,result),'down');
  result.status=ResultStatus.UP;assert.equal(health(monitor,result),'up');
  result.timestamp=now-1000n;assert.equal(health(monitor,result),'unknown');
  monitor.enabled=false;assert.equal(health(monitor,result),'paused');
});
