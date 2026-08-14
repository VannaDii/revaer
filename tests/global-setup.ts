import { execFileSync, spawn } from 'node:child_process';
import dotenv from 'dotenv';
import fs from 'node:fs';
import http from 'node:http';
import https from 'node:https';
import net from 'node:net';
import { randomBytes } from 'node:crypto';
import os from 'node:os';
import path from 'node:path';

import { cleanupE2EState } from './support/e2e-cleanup';
import { requireCoverage } from './support/e2e-coverage-config';
import { writeState } from './support/e2e-state';
import { repoRoot } from './support/paths';

type UrlParts = {
  host: string;
  port: number;
};

type ProcessInfo = {
  pid: number;
  logPath: string;
};

type HttpWaitConfig = {
  attempts: number;
  intervalMs: number;
};

const LOCAL_HOSTS = new Set(['localhost', '127.0.0.1', 'host.docker.internal']);

const STOP_PATTERNS = [
  'cargo run -p revaer-app',
  'cargo run -p revaer-ui',
  'trunk serve',
  'target/debug/revaer-app',
  'target/release/revaer-app',
];

const KNOWN_DEV_PROCESS = /revaer-app|revaer-ui|trunk serve|cargo run -p revaer-app|cargo run -p revaer-ui/;
const LOCAL_TEST_DB_USER = 'revaer';

const CARGO_BIN_DIR = process.env.CARGO_HOME
  ? path.join(process.env.CARGO_HOME, 'bin')
  : path.join(os.homedir(), '.cargo', 'bin');

const COMMAND_CANDIDATES = new Map<string, string[]>([
  [
    'cargo',
    [path.join(CARGO_BIN_DIR, 'cargo'), '/usr/local/bin/cargo', '/opt/homebrew/bin/cargo', '/usr/bin/cargo'],
  ],
  [
    'just',
    [path.join(CARGO_BIN_DIR, 'just'), '/usr/local/bin/just', '/opt/homebrew/bin/just', '/usr/bin/just'],
  ],
  [
    'lsof',
    [
      '/usr/sbin/lsof',
      '/usr/bin/lsof',
      '/usr/local/sbin/lsof',
      '/usr/local/bin/lsof',
      '/opt/homebrew/sbin/lsof',
      '/opt/homebrew/bin/lsof',
    ],
  ],
  [
    'pgrep',
    ['/usr/bin/pgrep', '/bin/pgrep', '/usr/local/bin/pgrep', '/opt/homebrew/bin/pgrep'],
  ],
  ['ps', ['/bin/ps', '/usr/bin/ps']],
  [
    'rustup',
    [
      path.join(CARGO_BIN_DIR, 'rustup'),
      '/usr/local/bin/rustup',
      '/opt/homebrew/bin/rustup',
      '/usr/bin/rustup',
    ],
  ],
  [
    'sqlx',
    [path.join(CARGO_BIN_DIR, 'sqlx'), '/usr/local/bin/sqlx', '/opt/homebrew/bin/sqlx', '/usr/bin/sqlx'],
  ],
  [
    'trunk',
    [
      path.join(CARGO_BIN_DIR, 'trunk'),
      '/usr/local/bin/trunk',
      '/opt/homebrew/bin/trunk',
      '/usr/bin/trunk',
    ],
  ],
]);

export default async function globalSetup(): Promise<void> {
  const root = repoRoot();
  const testsDir = path.join(root, 'tests');
  dotenv.config({ path: path.join(testsDir, '.env') });
  if (!process.env.REVAER_E2E_STATE_KEY) {
    process.env.REVAER_E2E_STATE_KEY = randomBytes(32).toString('hex');
  }

  try {
    const apiBaseUrl = process.env.E2E_API_BASE_URL ?? 'http://localhost:7070';
    const baseUrl = process.env.E2E_BASE_URL ?? 'http://localhost:8080';
    const requireUi = requireCoverage('UI');
    const uiPort = requireUi ? httpPortFromUrl(baseUrl) : undefined;
    const dbAdminUrl =
      process.env.E2E_DB_ADMIN_URL ??
      process.env.REVAER_TEST_DATABASE_URL ??
      defaultLocalDbAdminUrl();
    const dbPrefix = process.env.E2E_DB_PREFIX ?? 'revaer_e2e';
    const fsRoot = process.env.E2E_FS_ROOT ?? root;
    const resolvedFsRoot = path.isAbsolute(fsRoot) ? fsRoot : path.resolve(root, fsRoot);
    const mediaWorkspaceRoot = path.join(
      testsDir,
      '.runtime',
      `media-workspace-${process.pid}-${randomBytes(8).toString('hex')}`,
    );

    process.env.E2E_API_BASE_URL = apiBaseUrl;
    process.env.E2E_BASE_URL = baseUrl;
    process.env.E2E_DB_ADMIN_URL = dbAdminUrl;
    process.env.E2E_DB_PREFIX = dbPrefix;
    process.env.E2E_FS_ROOT = fsRoot;

    stopDevServers();
    await requirePortFree(7070);
    if (uiPort !== undefined) {
      await requirePortFree(uiPort);
    }
    fs.mkdirSync(resolvedFsRoot, { recursive: true });
    fs.mkdirSync(mediaWorkspaceRoot, { recursive: true });

    const adminUrl = await resolveAdminUrl(dbAdminUrl);
    const adminHost = urlParts(adminUrl).host;
    if (LOCAL_HOSTS.has(adminHost) && !isTruthy(process.env.E2E_SKIP_DB_START)) {
      const dbStartUrl = withPath(adminUrl, '/revaer');
      runCommandWithEnv(
        'just',
        ['db-start'],
        { DATABASE_URL: dbStartUrl, REVAER_DB_MANAGED: '1' },
        { cwd: root },
      );
    }
    runCommand('just', ['sqlx-install'], { cwd: root });

    runCommand('cargo', ['build', '-p', 'revaer-app'], { cwd: root });
    const apiBin = cargoDebugBinary(root, 'revaer-app');
    if (!fs.existsSync(apiBin)) {
      throw new Error(`revaer-app binary not found at ${apiBin}`);
    }

    const logDir = path.join(testsDir, 'logs');
    fs.mkdirSync(logDir, { recursive: true });

    const activeDbUrl = await createTempDb(adminUrl, dbPrefix, root);
    const apiProcess = spawnLoggedWithEnv(
      apiBin,
      [],
      path.join(logDir, 'api.log'),
      {
        DATABASE_URL: activeDbUrl,
        REVAER_MEDIA_WORKSPACE_ROOT: mediaWorkspaceRoot,
      },
      { cwd: root },
    );
    writeState({
      apiPid: apiProcess.pid,
      dbUrl: activeDbUrl,
      mediaWorkspaceRoot,
    });

    assertApiDb(apiProcess.pid, activeDbUrl);
    const httpWait = httpWaitConfig();
    await waitForHttp(`${apiBaseUrl}/health`, httpWait, apiProcess, 'API');
    assertApiListener(apiProcess.pid, 7070);

    if (!requireUi || uiPort === undefined) {
      return;
    }

    runCommand('just', ['sync-assets'], { cwd: root });
    runCommand('rustup', ['target', 'add', 'wasm32-unknown-unknown'], { cwd: root });
    const trunkCommand = requireCommand('trunk');
    fs.mkdirSync(path.join(root, 'crates', 'revaer-ui', 'dist-serve', '.stage'), {
      recursive: true,
    });

    const uiProcess = spawnLogged(
      trunkCommand,
      ['serve', '--dist', 'dist-serve', '--port', String(uiPort)],
      path.join(logDir, 'ui.log'),
      {
        cwd: path.join(root, 'crates', 'revaer-ui'),
      },
      {
        DATABASE_URL: activeDbUrl,
        REVAER_UI_API_BASE_URL: apiBaseUrl,
        RUST_LOG: process.env.RUST_LOG ?? 'info',
        NO_COLOR: 'true',
      },
    );

    await waitForHttp(baseUrl, httpWait, uiProcess, 'UI');

    writeState({
      apiPid: apiProcess.pid,
      dbUrl: activeDbUrl,
      mediaWorkspaceRoot,
      uiPid: uiProcess.pid,
    });
  } catch (error) {
    await cleanupE2EState();
    throw error;
  }
}

function isTruthy(value: string | undefined): boolean {
  if (!value) {
    return false;
  }
  return ['1', 'true', 'yes', 'on'].includes(value.toLowerCase());
}

function runCommand(
  command: string,
  args: string[],
  options?: { cwd?: string },
): void {
  execFileSync(requireCommand(command), args, {
    stdio: 'inherit',
    cwd: options?.cwd,
  });
}

function runCommandWithEnv(
  command: string,
  args: string[],
  overrides: NodeJS.ProcessEnv,
  options?: { cwd?: string },
): void {
  withTemporaryEnv(overrides, () => runCommand(command, args, options));
}

function spawnLogged(
  command: string,
  args: string[],
  logPath: string,
  options: { cwd?: string },
  overrides?: NodeJS.ProcessEnv,
): ProcessInfo {
  const out = fs.openSync(logPath, 'a');
  const child = withTemporaryEnv(overrides ?? {}, () =>
    spawn(command, args, {
      cwd: options.cwd,
      detached: true,
      stdio: ['ignore', out, out],
    }),
  );
  if (!child.pid) {
    throw new Error(`Failed to start ${command}.`);
  }
  child.unref();
  return { pid: child.pid, logPath };
}

function spawnLoggedWithEnv(
  command: string,
  args: string[],
  logPath: string,
  overrides: NodeJS.ProcessEnv,
  options: { cwd?: string },
): ProcessInfo {
  return spawnLogged(command, args, logPath, options, overrides);
}

function withTemporaryEnv<T>(overrides: NodeJS.ProcessEnv, callback: () => T): T {
  const previous = new Map<string, string | undefined>();
  for (const [key, value] of Object.entries(overrides)) {
    previous.set(key, process.env[key]);
    if (value === undefined) {
      delete process.env[key];
    } else {
      process.env[key] = value;
    }
  }
  try {
    return callback();
  } finally {
    for (const [key, value] of previous) {
      if (value === undefined) {
        delete process.env[key];
      } else {
        process.env[key] = value;
      }
    }
  }
}

function commandExists(command: string): boolean {
  return findCommand(command) !== null;
}

function requireCommand(command: string): string {
  const resolved = findCommand(command);
  if (!resolved) {
    throw new Error(`Required command not found in fixed command candidates: ${command}`);
  }
  return resolved;
}

function findCommand(command: string): string | null {
  if (path.isAbsolute(command) && fs.existsSync(command)) {
    return command;
  }
  const candidates = COMMAND_CANDIDATES.get(command) ?? [];
  return candidates.find((candidate) => fs.existsSync(candidate)) ?? null;
}

function cargoDebugBinary(root: string, binaryName: string): string {
  const targetDir = cargoTargetDir(root);
  const buildTarget = process.env.CARGO_BUILD_TARGET?.trim();
  const debugDir = buildTarget
    ? path.join(targetDir, buildTarget, 'debug')
    : path.join(targetDir, 'debug');
  return path.join(debugDir, executableName(binaryName));
}

function cargoTargetDir(root: string): string {
  const configured = process.env.CARGO_TARGET_DIR?.trim();
  if (!configured) {
    return path.join(root, 'target');
  }
  return path.isAbsolute(configured) ? configured : path.resolve(root, configured);
}

function executableName(binaryName: string): string {
  return process.platform === 'win32' ? `${binaryName}.exe` : binaryName;
}

function urlParts(input: string): UrlParts {
  const parsed = new URL(input);
  return {
    host: parsed.hostname,
    port: parsed.port ? Number(parsed.port) : 5432,
  };
}

function httpPortFromUrl(input: string): number {
  const parsed = new URL(input);
  if (parsed.port) {
    return Number(parsed.port);
  }
  if (parsed.protocol === 'https:') {
    return 443;
  }
  return 80;
}

function withPath(input: string, pathname: string): string {
  const parsed = new URL(input);
  parsed.pathname = pathname.startsWith('/') ? pathname : `/${pathname}`;
  return parsed.toString();
}

function defaultLocalDbAdminUrl(): string {
  const parsed = new URL('postgres://localhost:5432/postgres');
  parsed.username = LOCAL_TEST_DB_USER;
  parsed.password = LOCAL_TEST_DB_USER;
  return parsed.toString();
}

async function resolveAdminUrl(initial: string): Promise<string> {
  if (await dbUrlReachable(initial)) {
    return initial;
  }
  const fallbackUrls = [
    ...localHostVariants(initial),
    process.env.REVAER_TEST_DATABASE_URL,
    process.env.DATABASE_URL,
  ].filter(Boolean) as string[];
  for (const candidate of fallbackUrls) {
    if (await dbUrlReachable(candidate)) {
      return candidate;
    }
  }
  return initial;
}

function isLocalHost(host: string): boolean {
  return LOCAL_HOSTS.has(host);
}

function localHostVariants(initial: string): string[] {
  const host = urlParts(initial).host;
  if (!isLocalHost(host)) {
    return [];
  }
  const variants = ['localhost', '127.0.0.1', 'host.docker.internal']
    .filter((candidate) => candidate !== host)
    .map((candidate) => withHost(initial, candidate));
  return variants;
}

function withHost(input: string, host: string): string {
  const parsed = new URL(input);
  parsed.hostname = host;
  return parsed.toString();
}

async function dbUrlReachable(url: string): Promise<boolean> {
  const { host, port } = urlParts(url);
  return canConnect(host, port, 1000);
}

async function canConnect(host: string, port: number, timeoutMs: number): Promise<boolean> {
  return new Promise((resolve) => {
    const socket = net.createConnection({ host, port });
    const onDone = (result: boolean) => {
      socket.removeAllListeners();
      socket.destroy();
      resolve(result);
    };
    socket.setTimeout(timeoutMs, () => onDone(false));
    socket.once('error', () => onDone(false));
    socket.once('connect', () => onDone(true));
  });
}

async function isPortOpen(port: number): Promise<boolean> {
  return canConnect('127.0.0.1', port, 200);
}

function stopDevServers(): void {
  if (!commandExists('pgrep')) {
    return;
  }
  for (const pattern of STOP_PATTERNS) {
    stopDevServersMatching(pattern);
  }
}

function stopDevServersMatching(pattern: string): void {
  const output = pgrep(pattern);
  if (!output) {
    return;
  }
  for (const pidStr of output.split(/\s+/)) {
    stopDevPid(Number(pidStr));
  }
}

function pgrep(pattern: string): string {
  const pgrepPath = findCommand('pgrep');
  if (!pgrepPath) {
    return '';
  }
  try {
    return execFileSync(pgrepPath, ['-f', pattern], { encoding: 'utf-8' }).trim();
  } catch {
    return '';
  }
}

function stopDevPid(pid: number): void {
  if (!pid || pid === process.pid) {
    return;
  }
  const cmd = readCommand(pid);
  if (!shouldStopDevCommand(cmd)) {
    return;
  }
  console.error(`Stopping existing Revaer dev process (pid ${pid}: ${cmd})`);
  try {
    process.kill(pid, 'SIGTERM');
  } catch {
    // Ignore missing process.
  }
}

function shouldStopDevCommand(cmd: string): boolean {
  return Boolean(
    cmd &&
      !cmd.includes('pgrep -f') &&
      !cmd.includes('ps -p') &&
      !cmd.includes('global-setup'),
  );
}

async function requirePortFree(port: number): Promise<void> {
  if (!(await isPortOpen(port))) {
    return;
  }
  const freed = await stopKnownDevProcesses(port);
  if (freed) {
    return;
  }
  throw new Error(`Port ${port} is in use; stop existing services before running ui-e2e.`);
}

async function stopKnownDevProcesses(port: number): Promise<boolean> {
  if (!commandExists('lsof')) {
    return false;
  }
  const pids = pidsOnPort(port);
  if (pids.length === 0) {
    return false;
  }

  if (!stopKnownPids(port, pids)) {
    return false;
  }
  return waitForPortRelease(port);
}

function stopKnownPids(port: number, pids: number[]): boolean {
  let stopped = false;
  for (const pid of pids) {
    stopped = stopKnownPid(port, pid) || stopped;
  }
  return stopped;
}

function stopKnownPid(port: number, pid: number): boolean {
  const cmd = readCommand(pid);
  if (!cmd) {
    return false;
  }
  if (!KNOWN_DEV_PROCESS.test(cmd)) {
    throw new Error(`Port ${port} is in use by a non-Revaer process: ${cmd}`);
  }
  console.error(`Stopping existing Revaer dev process on port ${port} (pid ${pid}: ${cmd})`);
  try {
    process.kill(pid, 'SIGTERM');
    return true;
  } catch {
    return false;
  }
}

async function waitForPortRelease(port: number): Promise<boolean> {
  for (let attempt = 0; attempt < 20; attempt += 1) {
    if (!(await isPortOpen(port))) {
      return true;
    }
    await delay(250);
  }
  return false;
}

function pidsOnPort(port: number): number[] {
  const lsofPath = findCommand('lsof');
  if (!lsofPath) {
    return [];
  }
  try {
    const output = execFileSync(lsofPath, ['-ti', `:${port}`], {
      encoding: 'utf-8',
    }).trim();
    if (!output) {
      return [];
    }
    return output
      .split(/\s+/)
      .map(Number)
      .filter((pid) => Number.isFinite(pid) && pid > 0);
  } catch {
    return [];
  }
}

function readCommand(pid: number): string {
  const psPath = findCommand('ps');
  if (!psPath) {
    return '';
  }
  try {
    return execFileSync(psPath, ['-p', String(pid), '-o', 'args='], {
      encoding: 'utf-8',
    }).trim();
  } catch {
    return '';
  }
}

async function waitForHttp(
  url: string,
  config: HttpWaitConfig,
  processInfo?: ProcessInfo,
  label?: string,
): Promise<void> {
  for (let attempt = 0; attempt < config.attempts; attempt += 1) {
    if (await httpReady(url)) {
      return;
    }
    if (processInfo && !isPidAlive(processInfo.pid)) {
      const name = label ?? 'Process';
      throw new Error(
        `${name} (pid ${processInfo.pid}) exited before ${url} was ready. Check ${processInfo.logPath}.`,
      );
    }
    await delay(config.intervalMs);
  }
  const logHint = processInfo ? ` Check ${processInfo.logPath}.` : '';
  throw new Error(`Timed out waiting for ${url}.${logHint}`);
}

async function httpReady(url: string): Promise<boolean> {
  return new Promise((resolve) => {
    const target = new URL(url);
    const client = target.protocol === 'https:' ? https : http;
    const request = client.request(
      {
        protocol: target.protocol,
        hostname: target.hostname,
        port: target.port,
        path: `${target.pathname}${target.search}`,
        method: 'GET',
      },
      (response) => {
        response.resume();
        resolve((response.statusCode ?? 500) < 400);
      },
    );
    request.on('error', () => resolve(false));
    request.setTimeout(1000, () => {
      request.destroy();
      resolve(false);
    });
    request.end();
  });
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function parsePositiveInt(value: string | undefined): number | null {
  if (!value) {
    return null;
  }
  const parsed = Number.parseInt(value, 10);
  if (!Number.isFinite(parsed) || parsed <= 0) {
    return null;
  }
  return parsed;
}

function httpWaitConfig(): HttpWaitConfig {
  const intervalMs = parsePositiveInt(process.env.E2E_HTTP_WAIT_INTERVAL_MS) ?? 500;
  const attemptsOverride = parsePositiveInt(process.env.E2E_HTTP_WAIT_ATTEMPTS);
  if (attemptsOverride) {
    return { attempts: attemptsOverride, intervalMs };
  }
  const waitSeconds = parsePositiveInt(process.env.E2E_HTTP_WAIT_SECONDS) ?? 120;
  const attempts = Math.max(1, Math.ceil((waitSeconds * 1000) / intervalMs));
  return { attempts, intervalMs };
}

function isPidAlive(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch (error) {
    const code = typeof error === 'object' && error ? (error as NodeJS.ErrnoException).code : undefined;
    return code === 'EPERM';
  }
}

async function createTempDb(adminUrl: string, prefix: string, root: string): Promise<string> {
  const runId = `${Date.now()}_${randomBytes(4).toString('hex')}`;
  const dbName = `${prefix}_${runId}`;
  const dbUrl = withPath(adminUrl, dbName);
  runCommand(
    'sqlx',
    ['database', 'create', '--database-url', dbUrl],
    { cwd: root },
  );
  try {
    runCommandWithEnv(
      'cargo',
      ['run', '--quiet', '-p', 'revaer-data', '--bin', 'revaer-db-init'],
      { DATABASE_URL: dbUrl },
      { cwd: root },
    );
  } catch (error) {
    try {
      runCommand('sqlx', ['database', 'drop', '--database-url', dbUrl, '-y'], { cwd: root });
    } catch (cleanupError) {
      console.error('Failed to clean up temporary E2E database after initialization failure.', cleanupError);
    }
    throw error;
  }
  return dbUrl;
}

function assertApiDb(pid: number, expected: string): void {
  const environPath = `/proc/${pid}/environ`;
  if (!fs.existsSync(environPath)) {
    return;
  }
  const raw = fs.readFileSync(environPath);
  const envLines = raw.toString('utf-8').split('\0');
  const entry = envLines.find((line) => line.startsWith('DATABASE_URL='));
  if (!entry) {
    return;
  }
  const actual = entry.replace('DATABASE_URL=', '');
  if (actual && actual !== expected) {
    throw new Error(`API process started with unexpected DATABASE_URL: ${actual}`);
  }
}

function assertApiListener(pid: number, port: number): void {
  const lsofPath = findCommand('lsof');
  if (!lsofPath) {
    return;
  }
  try {
    const listener = execFileSync(lsofPath, ['-tiTCP:' + port, '-sTCP:LISTEN'], {
      encoding: 'utf-8',
    })
      .trim()
      .split(/\s+/)[0];
    if (listener && Number(listener) !== pid) {
      throw new Error(`Port ${port} is already bound by pid ${listener}; expected ${pid}.`);
    }
  } catch {
    // Ignore missing listeners.
  }
}
