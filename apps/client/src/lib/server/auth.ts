import { createHmac, randomBytes, timingSafeEqual } from 'node:crypto';

const maxAge = 8 * 60 * 60;
export const sessionCookie = 'uppe_session';
export const sessionMaxAge = maxAge;

export function operatorToken(): string {
  const token = process.env.UPPE_OPERATOR_TOKEN;
  if (!token || token.length < 32) throw new Error('Operator credential is not configured');
  return token;
}

function equal(a: string, b: string): boolean {
  const left = Buffer.from(a);
  const right = Buffer.from(b);
  return left.length === right.length && timingSafeEqual(left, right);
}

export function authenticate(token: string): boolean {
  return token.length <= 256 && equal(token, operatorToken());
}

function signature(payload: string): string {
  return createHmac('sha256', operatorToken()).update(`uppe/session/v1:${payload}`).digest('base64url');
}

export function issueSession(): string {
  const payload = `${Math.floor(Date.now() / 1000) + maxAge}.${randomBytes(24).toString('base64url')}`;
  return `${payload}.${signature(payload)}`;
}

export function validSession(value?: string): boolean {
  if (!value || value.length > 256) return false;
  const parts = value.split('.');
  if (parts.length !== 3) return false;
  const expires = Number(parts[0]);
  const now = Math.floor(Date.now() / 1000);
  return Number.isSafeInteger(expires) && expires > now && expires <= now + maxAge &&
    equal(parts[2], signature(`${parts[0]}.${parts[1]}`));
}

// Bounded storage and per-source throttling; forwarded headers are never trusted.
const attempts = new Map<string, { count: number; until: number }>();
export function allowLogin(address: string): boolean {
  const now = Date.now();
  for (const [key, value] of attempts) if (value.until < now) attempts.delete(key);
  const current = attempts.get(address) ?? { count: 0, until: now + 60_000 };
  if (!attempts.has(address) && attempts.size >= 1024) return false;
  attempts.set(address, current);
  return ++current.count <= 5;
}
