import { createClient } from '@connectrpc/connect';
import { createConnectTransport } from '@connectrpc/connect-web';
import { MonitorService } from '../../gen/monitor/v1/monitor_pb';
import { NetworkService } from '../../gen/network/v1/network_pb';
import { ResultService } from '../../gen/result/v1/result_pb';
import { SettingsService } from '../../gen/settings/v1/settings_pb';
import { StatusPageService } from '../../gen/statuspage/v1/statuspage_pb';
import { PublicStatusPageService } from '../../gen/publicstatuspage/v1/publicstatuspage_pb';

const baseUrl = import.meta.env.SSR
  ? process.env.UPPE_API_URL ?? 'http://127.0.0.1:8080'
  : new URL('/api', window.location.origin).href;
const transport = createConnectTransport({
  baseUrl,
  defaultTimeoutMs: 15_000,
  fetch: (input, init) => {
    const headers = new Headers(init?.headers);
    if (import.meta.env.SSR) {
      const token = process.env.UPPE_OPERATOR_TOKEN;
      if (token) headers.set('Authorization', 'Bearer ' + token);
    }
    return fetch(input, { ...init, headers, redirect: 'error' });
  },
});
export const monitorClient = createClient(MonitorService, transport);
export const networkClient = createClient(NetworkService, transport);
export const resultClient = createClient(ResultService, transport);
export const settingsClient = createClient(SettingsService, transport);
export const statusPageClient = createClient(StatusPageService, transport);
export const publicStatusPageClient = createClient(PublicStatusPageService, transport);
