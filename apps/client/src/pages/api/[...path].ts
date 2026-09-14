import type { APIRoute } from 'astro';
import { operatorToken } from '../../lib/server/auth';

const services = new Set(['monitor.v1.MonitorService', 'result.v1.ResultService', 'settings.v1.SettingsService', 'network.v1.NetworkService', 'statuspage.v1.StatusPageService']);

export const POST: APIRoute = async ({ request, params }) => {
  const path = params.path ?? '';
  const [service, method, extra] = path.split('/');
  if (!services.has(service) || !/^[A-Za-z]+$/.test(method ?? '') || extra !== undefined) return new Response(null, { status: 404 });
  // Read incrementally so chunked requests cannot bypass the body limit.
  const reader = request.body?.getReader();
  const chunks: Uint8Array[] = [];
  let size = 0;
  if (reader) {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      size += value.byteLength;
      if (size > 1_048_576) { await reader.cancel(); return new Response(null, { status: 413 }); }
      chunks.push(value);
    }
  }
  const body = Buffer.concat(chunks);
  const headers = new Headers({ Authorization: `Bearer ${operatorToken()}` });
  for (const key of ['content-type', 'connect-protocol-version']) {
    const value = request.headers.get(key); if (value) headers.set(key, value);
  }
  try {
    const base = process.env.UPPE_API_URL ?? 'http://127.0.0.1:8080';
    const result = await fetch(`${base}/${path}`, { method: 'POST', headers, body, redirect: 'error', signal: AbortSignal.timeout(15_000) });
    return new Response(result.body, { status: result.status, headers: { 'Content-Type': result.headers.get('Content-Type') ?? 'application/json', 'Cache-Control': 'no-store' } });
  } catch {
    return new Response(JSON.stringify({ code: 'unavailable', message: 'Your node is unavailable. Try again shortly.' }), { status: 503, headers: { 'Content-Type': 'application/json' } });
  }
};
