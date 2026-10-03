import { spawn, execFileSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { createInterface } from 'node:readline';
import { test as apiTest, expect } from './api';
import { createApiClient } from '../support/api/client';
import { repoRoot } from '../support/paths';
import type { ApiSession, AuthMode } from '../support/session';

type MediaService = { baseUrl: string; container: string; database: string; session: ApiSession };

function profileRequest(key: string) {
  return { profile_key: key, display_name: 'Owned profile fixture', description: '', enabled: true,
    dry_run_only: true, desired_target_key: key, desired_target_version: 1, policy_key: key,
    policy_version: 1, output_root_key: 'source', workspace_root_key: 'workspace',
    quarantine_root_key: 'quarantine' };
}

type ProfileFixture = {
  request: ReturnType<typeof profileRequest>; prefix: string; sourcePath: string;
  relativePath: string; sourceBytes: Buffer; readSource: () => Buffer;
};

async function bounded<T>(promise: Promise<T>, milliseconds: number, stage: string): Promise<T> {
  let timer: NodeJS.Timeout | undefined;
  try {
    return await Promise.race([promise, new Promise<T>((_, reject) => {
      timer = setTimeout(() => reject(new Error(`Media fixture ${stage} timed out`)), milliseconds);
    })]);
  } finally {
    clearTimeout(timer);
  }
}

export const test = apiTest.extend<{ profileFixture: ProfileFixture }, { mediaService: MediaService }>({
  profileFixture: async ({ api, mediaService }, use) => {
    const prefix = `test-${randomUUID()}`;
    const root = `/proof/source/${prefix}`;
    const sourcePath = `${root}/source.mkv`;
    const docker = (...args: string[]) => execFileSync('docker', ['exec', mediaService.container, ...args]);
    try {
      docker('mkdir', '-p', root);
      docker('cp', '/proof/source/Movies/original.mkv', sourcePath);
      const readSource = () => docker('cat', sourcePath);
      const sourceBytes = readSource();
      const target = await api.POST('/v1/media/targets', { body: {
        target_key: prefix, version: 1, display_name: 'Owned target', container_format: 'matroska',
        streams: [{ stream_key: 'video', stream_kind: 'video', optional: false, sort_order: 0,
          codec: 'h264', default_disposition: true, forced_disposition: false }],
      } });
      expect(target.response.status, JSON.stringify(target.error)).toBe(201);
      const policy = await api.POST('/v1/media/policies', { body: {
        policy_key: prefix, version: 1, display_name: 'Owned dry-run policy', video_intent: 'general',
        verification_strictness: 'strict', verification_duration_tolerance_millis: 100,
        verification_mux_validation: true, verification_decode_all_streams: true,
        verification_keyframe_seek: true, verification_playback_probe: true,
        output: { dry_run: true, replacement_mode: 'atomic_replace', quarantine_enabled: true,
          preserve_permissions: true, preserve_ownership: true },
      } });
      expect(policy.response.status, JSON.stringify(policy.error)).toBe(201);
      await use({ request: profileRequest(prefix), prefix, sourcePath,
        relativePath: `${prefix}/source.mkv`, sourceBytes, readSource });
    } finally { docker('rm', '-rf', '--', root); }
  },
  mediaService: [async ({}, use, workerInfo) => {
    const authMode = workerInfo.project.metadata.authMode as AuthMode;
    if (!['none', 'api_key'].includes(authMode)) throw new Error('Missing media fixture authentication mode');
    const child = spawn('just', ['--command', 'bash', 'scripts/with-media-test-service.sh',
      'bash', 'scripts/with-node.sh', 'node', 'tests/support/media-api-service.cjs'], {
      cwd: repoRoot(), detached: true, stdio: ['pipe', 'pipe', 'pipe'],
      env: { ...process.env, E2E_MEDIA_AUTH_MODE: authMode },
    });
    const stopped = new Promise<{ code: number | null; signal: NodeJS.Signals | null }>(resolve => {
      child.once('close', (code, signal) => resolve({ code, signal }));
    });
    const lines = createInterface({ input: child.stdout });
    let diagnostics = '';
    child.stderr.on('data', chunk => { diagnostics = (diagnostics + chunk.toString()).slice(-4000); });
    const ready = new Promise<MediaService>((resolve, reject) => {
      child.once('error', reject);
      child.once('close', code => reject(new Error(`Media fixture exited before readiness: ${code}\n${diagnostics}`)));
      lines.on('line', line => {
        if (!line.startsWith('MEDIA_API_FIXTURE_READY ')) return;
        try {
          const value = JSON.parse(line.slice('MEDIA_API_FIXTURE_READY '.length)) as MediaService;
          const url = new URL(value.baseUrl);
          if (url.protocol !== 'http:' || url.hostname !== '127.0.0.1' || !url.port ||
            !/^revaer-root-positive-[0-9]+-app$/.test(value.container) ||
            !/^revaer_test_[0-9]+_[0-9]+$/.test(value.database) ||
            value.session.authMode !== authMode ||
            (authMode === 'api_key' && !value.session.apiKey)) {
            throw new Error('Invalid owned media fixture readiness');
          }
          resolve(value);
        } catch (error) { reject(error); }
      });
    });
    try {
      await use(await bounded(ready, 120000, 'startup'));
    } finally {
      child.stdin.end();
      try {
        const result = await bounded(stopped, 60000, 'teardown');
        if (result.code !== 0) throw new Error(`Media fixture teardown failed: ${result.code}/${result.signal}\n${diagnostics}`);
      } catch (error) {
        if (child.exitCode === null && child.signalCode === null && child.pid) {
          process.kill(-child.pid, 'SIGTERM');
          await bounded(stopped, 30000, 'forced teardown');
        }
        throw error;
      } finally { lines.close(); }
    }
  }, { scope: 'worker', timeout: 210000 }],
  session: [async ({ _apiCoverage, mediaService }, use) => {
    void _apiCoverage;
    await use(mediaService.session);
  }, { scope: 'worker' }],
  baseUrl: async ({ mediaService }, use) => { await use(mediaService.baseUrl); },
  api: async ({ mediaService, session }, use) => {
    await use(createApiClient({ baseUrl: mediaService.baseUrl,
      headers: session.apiKey ? { 'x-revaer-api-key': session.apiKey } : undefined }));
  },
  publicApi: async ({ mediaService }, use) => {
    await use(createApiClient({ baseUrl: mediaService.baseUrl }));
  },
});

export { expect };
