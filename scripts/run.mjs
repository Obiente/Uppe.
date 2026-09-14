import { spawn } from 'node:child_process';
import { access } from 'node:fs/promises';
import { join } from 'node:path';
import { root, runtimeEnvironment } from './runtime.mjs';

const mode = process.argv[2] || 'start';
const { env, data, accessPath, config } = await runtimeEnvironment(process.env, { credentials: mode === 'start' });
Object.assign(process.env, env);
const windows = process.platform === 'win32';
const profile = process.argv.includes('--debug') ? 'debug' : 'release';
const service = join(root, 'target', profile, 'uppe-service' + (windows ? '.exe' : ''));
const api = join(data, 'bin', 'uppe-api' + (windows ? '.exe' : ''));

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

if (mode === 'build') {
  await run('cargo', ['build', '--locked', '--bin', 'uppe-service', ...(profile === 'release' ? ['--release'] : [])]);
  await run('go', ['build', '-trimpath', '-o', api, './cmd/uppe-api'], join(root, 'apps/server'));
  await pnpm(['--filter', '@uppe/client', 'build']);
  console.log('Build complete. Start with pnpm start' + (profile === 'debug' ? ' --debug' : '') + '.');
} else if (mode === 'start') {
  await Promise.all([access(service), access(api), access(join(root, 'apps/client/dist/server/entry.mjs'))]).catch(() => {
    throw new Error('Build the application first with pnpm build' + (profile === 'debug' ? ' --debug' : '') + '.');
  });
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
