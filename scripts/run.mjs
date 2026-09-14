import { spawn } from 'node:child_process';
import { randomBytes } from 'node:crypto';
import { mkdir, readFile, writeFile, access } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const data = resolve(process.env.UPPE_DATA_DIR || join(root, '.uppe'));
const windows = process.platform === 'win32';
const profile = process.argv.includes('--debug') ? 'debug' : 'release';
const service = join(root, 'target', profile, 'uppe-service' + (windows ? '.exe' : ''));
const api = join(data, 'bin', 'uppe-api' + (windows ? '.exe' : ''));
await mkdir(join(data, 'bin'), { recursive: true, mode: 0o700 });
process.env.UPPE_DATA_DIR = data;
process.env.UPPE_DATABASE_PATH ||= join(data, 'uppe.db');
process.env.UPPE_KEYPAIR_PATH ||= join(data, 'node.key');
const config = ['--config', join(data, 'config.toml')];

function run(command, args, cwd = root) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { cwd, env: process.env, stdio: 'inherit', windowsHide: true });
    child.once('error', reject);
    child.once('exit', code => code === 0 ? resolve() : reject(new Error(`${command} exited with ${code}`)));
  });
}
async function pnpm(args) {
  if (!process.env.npm_execpath) throw new Error('Run this script through pnpm.');
  await run(process.execPath, [process.env.npm_execpath, ...args]);
}

const mode = process.argv[2] || 'start';
if (mode === 'build') {
  await run('cargo', ['build', '--locked', '--bin', 'uppe-service', ...(profile === 'release' ? ['--release'] : [])]);
  await run('go', ['build', '-trimpath', '-o', api, './cmd/uppe-api'], join(root, 'apps/server'));
  await pnpm(['--filter', '@uppe/client', 'build']);
  console.log('Build complete. Start with pnpm start' + (profile === 'debug' ? ' --debug' : '') + '.');
} else if (mode === 'start') {
  await Promise.all([access(service), access(api), access(join(root, 'apps/client/dist/server/entry.mjs'))]).catch(() => {
    throw new Error('Build the application first with pnpm build' + (profile === 'debug' ? ' --debug' : '') + '.');
  });
  const accessPath = join(data, 'access.key');
  if (!process.env.UPPE_OPERATOR_TOKEN) {
    try { await writeFile(accessPath, randomBytes(32).toString('hex') + '\n', { flag: 'wx', mode: 0o600 }); }
    catch (error) { if (error.code !== 'EEXIST') throw error; }
    process.env.UPPE_OPERATOR_TOKEN = (await readFile(accessPath, 'utf8')).trim();
  }
  if (process.env.UPPE_OPERATOR_TOKEN.length < 32) throw new Error('The access key must contain at least 32 characters.');
  process.env.HOST ||= '127.0.0.1';
  process.env.PORT ||= '4321';
  process.env.UPPE_API_ADDRESS ||= '127.0.0.1:8080';
  process.env.UPPE_API_URL ||= 'http://' + process.env.UPPE_API_ADDRESS;
  await run(service, [...config, 'migrate']);
  const children = [];
  let stopping = false;
  function stop(code = 0) {
    if (stopping) return;
    stopping = true; process.exitCode = code;
    for (const child of children) child.kill('SIGTERM');
  }
  for (const [command, args, cwd] of [
    [service, [...config, 'run'], root],
    [api, [], root],
    [process.execPath, ['dist/server/entry.mjs'], join(root, 'apps/client')],
  ]) {
    const child = spawn(command, args, { cwd, env: process.env, stdio: 'inherit', windowsHide: true });
    children.push(child);
    child.once('error', error => { console.error(error.message); stop(1); });
    child.once('exit', code => { if (!stopping) { console.error('A required process stopped. Shutting down the installation.'); stop(code || 1); } });
  }
  process.once('SIGINT', () => stop());
  process.once('SIGTERM', () => stop());
  console.log(`Uppe is starting at http://${process.env.HOST}:${process.env.PORT}.`);
  console.log(`Sign in using your configured access key or the key saved at ${accessPath}.`);
} else {
  throw new Error('Choose build or start.');
}
