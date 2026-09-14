import assert from 'node:assert/strict';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { root, runtimeEnvironment } from './runtime.mjs';

async function fixture(t) {
  const data = await mkdtemp(join(tmpdir(), 'uppe-bootstrap-'));
  t.after(() => rm(data, { recursive: true, force: true }));
  return { UPPE_DATA_DIR: data };
}

test('fresh bootstrap creates one persistent credential shared by all components', async t => {
  const source = await fixture(t);
  const first = await runtimeEnvironment(source);
  assert.match(first.env.UPPE_OPERATOR_TOKEN, /^[a-f0-9]{64}$/);
  const components = await Promise.all(Array.from({ length: 3 }, () => runtimeEnvironment(source, { createKey: false })));
  for (const { env } of components) assert.deepEqual(env, first.env);
  const restarted = await runtimeEnvironment(source);
  assert.equal(restarted.env.UPPE_OPERATOR_TOKEN, first.env.UPPE_OPERATOR_TOKEN);
  assert.equal(source.UPPE_OPERATOR_TOKEN, undefined);
});

test('external credentials and endpoint overrides are preserved without saving the token', async t => {
  const source = { ...await fixture(t), UPPE_OPERATOR_TOKEN: 'synthetic-external-test-token-123456789', UPPE_API_ADDRESS: '127.0.0.1:18080', PORT: '14321' };
  const { env, accessPath } = await runtimeEnvironment(source);
  assert.equal(env.UPPE_OPERATOR_TOKEN, source.UPPE_OPERATOR_TOKEN);
  assert.equal(env.UPPE_API_URL, 'http://127.0.0.1:18080');
  assert.equal(env.PORT, '14321');
  await assert.rejects(readFile(accessPath), { code: 'ENOENT' });
});

test('invalid saved or explicit credentials fail without silently rotating a key', async t => {
  const source = await fixture(t);
  const accessPath = join(source.UPPE_DATA_DIR, 'access.key');
  await writeFile(accessPath, 'invalid');
  await assert.rejects(runtimeEnvironment(source), /at least 32/);
  await assert.rejects(runtimeEnvironment({ ...source, UPPE_OPERATOR_TOKEN: 'short' }), /at least 32/);
  assert.equal(await readFile(accessPath, 'utf8'), 'invalid');
});

test('relative database and identity overrides use the workspace root', async t => {
  const { env } = await runtimeEnvironment({ ...await fixture(t), UPPE_DATABASE_PATH: '.uppe/alternate.db', UPPE_KEYPAIR_PATH: '.uppe/alternate.key' });
  assert.equal(env.UPPE_DATABASE_PATH, join(root, '.uppe/alternate.db'));
  assert.equal(env.UPPE_KEYPAIR_PATH, join(root, '.uppe/alternate.key'));
});

test('component startup requires completed bootstrap and does not invent another key', async t => {
  const source = await fixture(t);
  await assert.rejects(runtimeEnvironment(source, { createKey: false }), { code: 'ENOENT' });
});
