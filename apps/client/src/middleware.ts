import { defineMiddleware } from 'astro:middleware';
import { sessionCookie, validSession } from './lib/server/auth';

export const onRequest = defineMiddleware(async (context, next) => {
  const path = context.url.pathname;
  const publicRoute = path === '/' || path === '/login' || path.startsWith('/public/');
  if (!publicRoute && !validSession(context.cookies.get(sessionCookie)?.value)) {
    if (path.startsWith('/api/')) return new Response('Sign in to continue', { status: 401 });
    return context.redirect('/login');
  }
  if (!['GET', 'HEAD', 'OPTIONS'].includes(context.request.method) &&
      context.request.headers.get('origin') !== context.url.origin) {
    return new Response('Request origin is not allowed', { status: 403 });
  }
  const response = await next();
  response.headers.set('X-Content-Type-Options', 'nosniff');
  response.headers.set('Referrer-Policy', 'same-origin');
  response.headers.set('X-Frame-Options', 'DENY');
  response.headers.set('Cache-Control', 'no-store');
  return response;
});
