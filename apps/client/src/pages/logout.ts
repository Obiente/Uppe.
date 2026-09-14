import type { APIRoute } from 'astro';
import { sessionCookie } from '../lib/server/auth';
export const POST: APIRoute = ({ cookies, redirect }) => {
  cookies.delete(sessionCookie, { path: '/' });
  return redirect('/login', 303);
};
