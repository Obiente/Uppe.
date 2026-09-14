import { spawn } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { root, runtimeEnvironment } from './runtime.mjs';

const mode = process.argv[2];
if (!['bootstrap', 'service', 'server', 'client'].includes(mode)) throw new Error('Choose bootstrap, service, server or client.');
const { env, data, accessPath, config } = await runtimeEnvironment(process.env, { createKey: mode === 'bootstrap' });
const windows = process.platform === 'win32';
const target = resolve(root, env.CARGO_TARGET_DIR || 'target');
const service = join(target, 'debug', 'uppe-service' + (windows ? '.exe' : ''));
const api = join(data, 'bin', 'uppe-api-dev' + (windows ? '.exe' : ''));
let child;
let stopping = false;

function stop(signal) {
  stopping = true;
  process.exitCode = signal === 'SIGINT' ? 130 : 143;
  if (!child?.pid) return;
  if (windows) {
    // Include compiler and frontend worker descendants when Turbo cancels a task.
    spawn('taskkill', ['/pid', String(child.pid), '/T', '/F'], { stdio: 'ignore', windowsHide: true });
  } else {
    try { process.kill(-child.pid, signal); } catch (error) { if (error.code !== 'ESRCH') throw error; }
  }
}
process.once('SIGINT', () => stop('SIGINT'));
process.once('SIGTERM', () => stop('SIGTERM'));

async function run(command, args, cwd = root) {
  if (stopping) throw new Error('Development startup cancelled.');
  await new Promise((resolve, reject) => {
    child = spawn(command, args, { cwd, env, stdio: 'inherit', windowsHide: true, detached: !windows });
    child.once('error', reject);
    child.once('exit', (code, signal) => {
      child = undefined;
      if (code === 0 || stopping) resolve();
      else reject(new Error(`${command} exited with ${signal || code}`));
    });
  });
}

if (mode === 'bootstrap') {
  await run('cargo', ['build', '--locked', '--bin', 'uppe-service', '-j', '2']);
  await run('go', ['build', '-trimpath', '-o', api, './cmd/uppe-api'], join(root, 'apps/server'));
  await run(service, [...config, 'migrate']);
  console.log('Development database ready. Turbo can now start the application.');
  console.log(process.env.UPPE_OPERATOR_TOKEN ? 'Sign in with your configured operator token.' : `Sign in with the key in ${accessPath}.`);
} else if (mode === 'service') {
  await run(service, [...config, 'run']);
} else if (mode === 'server') {
  await run(api, []);
} else {
  const client = join(root, 'apps/client');
  const astro = join(client, 'node_modules/astro');
  const manifest = JSON.parse(await readFile(join(astro, 'package.json'), 'utf8'));
  await run(process.execPath, [join(astro, manifest.bin.astro), 'dev', '--host', env.HOST, '--port', env.PORT], client);
}
