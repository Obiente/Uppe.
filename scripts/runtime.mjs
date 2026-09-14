import { randomBytes } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

export const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');

// Resolve paths against the workspace, never a Turbo task's package directory.
export async function runtimeEnvironment(source = process.env, { credentials = true, createKey = true } = {}) {
  const env = { ...source };
  const data = resolve(root, env.UPPE_DATA_DIR || '.uppe');
  env.UPPE_DATA_DIR = data;
  env.UPPE_DATABASE_PATH = resolve(root, env.UPPE_DATABASE_PATH || join(data, 'uppe.db'));
  env.UPPE_KEYPAIR_PATH = resolve(root, env.UPPE_KEYPAIR_PATH || join(data, 'node.key'));
  env.HOST ||= '127.0.0.1';
  env.PORT ||= '4321';
  env.UPPE_API_ADDRESS ||= '127.0.0.1:8080';
  env.UPPE_API_URL ||= 'http://' + env.UPPE_API_ADDRESS;
  await mkdir(join(data, 'bin'), { recursive: true, mode: 0o700 });
  const accessPath = join(data, 'access.key');
  if (credentials) {
    if (!env.UPPE_OPERATOR_TOKEN) {
      if (createKey) {
        try {
          await writeFile(accessPath, randomBytes(32).toString('hex') + '\n', { flag: 'wx', mode: 0o600 });
        } catch (error) {
          if (error.code !== 'EEXIST') throw error;
        }
      }
      env.UPPE_OPERATOR_TOKEN = (await readFile(accessPath, 'utf8')).trim();
    }
    if (env.UPPE_OPERATOR_TOKEN.length < 32) {
      throw new Error('The operator access key must contain at least 32 characters. Set UPPE_OPERATOR_TOKEN or replace the invalid access.key.');
    }
  }
  return { env, data, accessPath, config: ['--config', join(data, 'config.toml')] };
}
