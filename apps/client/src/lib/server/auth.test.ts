import { test } from 'node:test';
import assert from 'node:assert/strict';
import { authenticate, issueSession, validSession, allowLogin } from './auth.ts';
import { boundedBody } from './body.ts';

test('sessions reject tampering, expiry and key rotation', () => {
  process.env.UPPE_OPERATOR_TOKEN = 'synthetic-test-key-'.repeat(4);
  assert.ok(authenticate(process.env.UPPE_OPERATOR_TOKEN));
  assert.ok(!authenticate('wrong'));
  const session=issueSession(); assert.ok(validSession(session));
  assert.ok(!validSession(session+'x')); assert.ok(!validSession('0.'+session.split('.').slice(1).join('.')));
  const before=Date.now; Date.now=()=>before()+9*60*60*1000;
  try { assert.ok(!validSession(session)); } finally { Date.now=before; }
  process.env.UPPE_OPERATOR_TOKEN='rotated-synthetic-key-'.repeat(4);
  assert.ok(!validSession(session));
  assert.ok(!validSession('x'.repeat(300)));
});
test('login throttling bounds attempts per source',()=>{
  for(let i=0;i<5;i++)assert.ok(allowLogin('synthetic-source'));
  assert.ok(!allowLogin('synthetic-source'));
});
test('chunked bodies enforce the actual byte limit', async()=>{
  const request=new Request('http://localhost/login',{method:'POST',body:'x'.repeat(4097)});
  await assert.rejects(boundedBody(request,4096));
  assert.equal((await boundedBody(new Request('http://localhost/login',{method:'POST',body:'token=test'}),4096)).length,10);
});
