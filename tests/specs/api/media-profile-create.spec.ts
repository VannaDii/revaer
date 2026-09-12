import { randomUUID } from 'node:crypto';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { test, expect } from '../../fixtures/api';

const rootsToRemove = new Set<string>();

test.afterEach(async () => {
  const roots = [...rootsToRemove];
  rootsToRemove.clear();
  await Promise.all(roots.map((root) => rm(root, { recursive: true, force: true })));
});

async function profileFixture() {
  const root = await mkdtemp(path.join(tmpdir(), 'revaer-profile-create-'));
  rootsToRemove.add(root);
  const sourceRoot = path.join(root, 'source');
  const outputRoot = path.join(root, 'output');
  await mkdir(sourceRoot);
  await mkdir(outputRoot);
  const sourcePath = path.join(sourceRoot, 'source.mkv');
  const sourceBytes = Buffer.from('profile-create source must remain unchanged');
  await writeFile(sourcePath, sourceBytes);
  return {
    sourcePath,
    sourceBytes,
    request: {
      profile_key: `profile-create-${randomUUID()}`,
      source_root: sourceRoot,
      output_root: outputRoot,
      dry_run_only: true,
      retention_days: 30,
      schedule_enabled: false,
      watcher_enabled: false,
    },
  };
}

test.describe('Media profile creation', () => {
  test('persists a new profile as dry-run with automation disabled', async ({ api }) => {
    const fixture = await profileFixture();
    const created = await api.POST('/v1/media/profiles', {
      body: { ...fixture.request, dry_run_only: false },
    });
    expect(created.response.status, JSON.stringify(created.error)).toBe(201);
    expect(created.data).toMatchObject({
      ...fixture.request,
      policy_key: 'safe_dry_run',
    });
    expect(created.data?.schedule_interval_minutes ?? null).toBeNull();
    const profileId = created.data?.media_profile_public_id;
    if (!profileId) {
      throw new Error('Missing media profile public id');
    }
    const persisted = await api.GET('/v1/media/profiles/{media_profile_public_id}', {
      params: { path: { media_profile_public_id: profileId } },
    });
    expect(persisted.response.status, JSON.stringify(persisted.error)).toBe(200);
    expect(persisted.data).toEqual(created.data);
    expect(await readFile(fixture.sourcePath)).toEqual(fixture.sourceBytes);
  });

  for (const automation of ['schedule', 'watcher'] as const) {
    test(`rejects ${automation} creation without verified filesystem identity`, async ({ api }) => {
      const fixture = await profileFixture();
      // ADR 419 keeps legacy path-taking creation fail-closed for automation.
      const rejected = await api.POST('/v1/media/profiles', {
        body: {
          ...fixture.request,
          ...(automation === 'schedule'
            ? { schedule_enabled: true, schedule_interval_minutes: 60 }
            : { watcher_enabled: true }),
        },
      });
      expect(rejected.response.status, JSON.stringify(rejected.error)).toBe(400);
      expect(rejected.error).toMatchObject({
        context: [
          { name: 'operation', value: 'media_profile_upsert' },
          { name: 'error_code', value: 'media_profile_filesystem_identity_required' },
          { name: 'sqlstate', value: 'P0001' },
        ],
      });
      const profiles = await api.GET('/v1/media/profiles');
      expect(profiles.response.status, JSON.stringify(profiles.error)).toBe(200);
      expect(
        profiles.data?.profiles.some((profile) => profile.profile_key === fixture.request.profile_key)
      ).toBe(false);
      expect(await readFile(fixture.sourcePath)).toEqual(fixture.sourceBytes);
    });
  }
});

test.describe('Media profile updates', () => {
  test('updates metadata without enabling automation or changing roots', async ({ api }) => {
    const fixture = await profileFixture();
    const created = await api.POST('/v1/media/profiles', { body: fixture.request });
    expect(created.response.status, JSON.stringify(created.error)).toBe(201);
    const profileId = created.data?.media_profile_public_id;
    if (!profileId) {
      throw new Error('Missing media profile public id');
    }
    const params = { path: { media_profile_public_id: profileId } };
    const patched = await api.PATCH('/v1/media/profiles/{media_profile_public_id}', {
      params,
      body: {
        source_root: fixture.request.source_root,
        output_root: fixture.request.output_root,
        dry_run_only: true,
        retention_days: 31,
        schedule_enabled: false,
        watcher_enabled: false,
      },
    });
    expect(patched.response.status, JSON.stringify(patched.error)).toBe(200);
    expect(patched.data).toMatchObject({ ...fixture.request, retention_days: 31 });
    expect(patched.data?.schedule_interval_minutes ?? null).toBeNull();
    const persisted = await api.GET('/v1/media/profiles/{media_profile_public_id}', { params });
    expect(persisted.response.status, JSON.stringify(persisted.error)).toBe(200);
    expect(persisted.data).toEqual(patched.data);
    expect(await readFile(fixture.sourcePath)).toEqual(fixture.sourceBytes);
  });

  const unverifiedUpdates = [
    { name: 'schedule enablement', body: { schedule_enabled: true, schedule_interval_minutes: 60 } },
    { name: 'interval alone', body: { schedule_interval_minutes: 120 } },
    {
      name: 'interval with scheduling disabled',
      body: { schedule_enabled: false, schedule_interval_minutes: 120 },
    },
    { name: 'watcher enablement', body: { watcher_enabled: true } },
  ];
  for (const update of unverifiedUpdates) {
    test(`rejects ${update.name} without verified filesystem identity`, async ({ api }) => {
      const fixture = await profileFixture();
      const created = await api.POST('/v1/media/profiles', { body: fixture.request });
      expect(created.response.status, JSON.stringify(created.error)).toBe(201);
      const profileId = created.data?.media_profile_public_id;
      if (!profileId) {
        throw new Error('Missing media profile public id');
      }
      const params = { path: { media_profile_public_id: profileId } };
      const rejected = await api.PATCH('/v1/media/profiles/{media_profile_public_id}', {
        params,
        body: { ...update.body, retention_days: 31 },
      });
      expect(rejected.response.status, JSON.stringify(rejected.error)).toBe(400);
      expect(rejected.error).toMatchObject({
        context: [
          { name: 'operation', value: 'media_profile_patch' },
          { name: 'error_code', value: 'media_profile_filesystem_identity_required' },
          { name: 'sqlstate', value: 'P0001' },
        ],
      });
      const persisted = await api.GET('/v1/media/profiles/{media_profile_public_id}', { params });
      expect(persisted.response.status, JSON.stringify(persisted.error)).toBe(200);
      expect(persisted.data).toEqual(created.data);
      expect(await readFile(fixture.sourcePath)).toEqual(fixture.sourceBytes);
    });
  }
});

test.describe('Dry-run profile discovery', () => {
  test('admits a manual job and reads its diagnostics without enabling automation', async ({ api }) => {
    const fixture = await profileFixture();
    const created = await api.POST('/v1/media/profiles', { body: fixture.request });
    expect(created.response.status, JSON.stringify(created.error)).toBe(201);
    const profileId = created.data?.media_profile_public_id;
    if (!profileId) {
      throw new Error('Missing media profile public id');
    }
    const profileParams = { path: { media_profile_public_id: profileId } };
    const readiness = await api.GET('/v1/media/profiles/{media_profile_public_id}/readiness', {
      params: profileParams,
    });
    expect(readiness.response.status, JSON.stringify(readiness.error)).toBe(200);
    expect(readiness.data?.profile.media_profile_public_id).toBe(profileId);
    expect(typeof readiness.data?.ready).toBe('boolean');

    const planning = await api.POST('/v1/media/planning/preview', {
      body: { media_profile_public_id: profileId, source_path: fixture.sourcePath },
    });
    expect(planning.response.status, JSON.stringify(planning.error)).toBe(200);
    expect(planning.data?.accepted).toBe(true);
    const body = { media_profile_public_id: profileId, source_paths: [fixture.sourcePath] };
    const preview = await api.POST('/v1/media/discovery/preview', { body });
    expect(preview.response.status, JSON.stringify(preview.error)).toBe(200);
    expect(preview.data?.previews[0]?.accepted).toBe(true);
    const run = await api.POST('/v1/media/discovery/runs', { body });
    expect(run.response.status, JSON.stringify(run.error)).toBe(201);
    expect(run.data?.skipped).toEqual([]);
    expect(run.data?.queued_jobs).toHaveLength(1);
    expect(run.data?.queued_jobs[0]).toMatchObject({
      source_path: fixture.sourcePath,
      dry_run: true,
    });
    const jobId = run.data?.queued_jobs[0]?.media_job_public_id;
    if (!jobId) {
      throw new Error('Missing admitted media job public id');
    }
    const duplicate = await api.POST('/v1/media/discovery/runs', { body });
    expect(duplicate.response.status, JSON.stringify(duplicate.error)).toBe(201);
    expect(duplicate.data?.queued_jobs).toEqual([]);
    expect(duplicate.data?.skipped).toEqual([
      { source_path: fixture.sourcePath, reason: 'media_discovery_source_unchanged' },
    ]);

    const jobs = await api.GET('/v1/media/jobs', {
      params: { query: { media_profile_public_id: profileId } },
    });
    expect(jobs.response.status, JSON.stringify(jobs.error)).toBe(200);
    expect(jobs.data?.jobs.map((job) => job.media_job_public_id)).toEqual([jobId]);
    const params = { path: { media_job_public_id: jobId } };
    const job = await api.GET('/v1/media/jobs/{media_job_public_id}', { params });
    expect(job.response.status, JSON.stringify(job.error)).toBe(200);
    expect(job.data?.source_path).toBe(fixture.sourcePath);

    const phases = await api.GET('/v1/media/jobs/{media_job_public_id}/phases', { params });
    expect(phases.response.status, JSON.stringify(phases.error)).toBe(200);
    expect(Array.isArray(phases.data?.phases)).toBe(true);
    const operations = await api.GET('/v1/media/jobs/{media_job_public_id}/operations', { params });
    expect(operations.response.status, JSON.stringify(operations.error)).toBe(200);
    expect(Array.isArray(operations.data?.operations)).toBe(true);
    const violations = await api.GET('/v1/media/jobs/{media_job_public_id}/violations', { params });
    expect(violations.response.status, JSON.stringify(violations.error)).toBe(200);
    expect(Array.isArray(violations.data?.violations)).toBe(true);
    const reasons = await api.GET('/v1/media/jobs/{media_job_public_id}/plan-reasons', { params });
    expect(reasons.response.status, JSON.stringify(reasons.error)).toBe(200);
    expect(Array.isArray(reasons.data?.reasons)).toBe(true);
    const checks = await api.GET('/v1/media/jobs/{media_job_public_id}/verification-checks', { params });
    expect(checks.response.status, JSON.stringify(checks.error)).toBe(200);
    expect(Array.isArray(checks.data?.checks)).toBe(true);
    const artifacts = await api.GET('/v1/media/jobs/{media_job_public_id}/artifacts', { params });
    expect(artifacts.response.status, JSON.stringify(artifacts.error)).toBe(200);
    expect(Array.isArray(artifacts.data?.artifacts)).toBe(true);
    const audits = await api.GET('/v1/media/jobs/{media_job_public_id}/compact-audits', { params });
    expect(audits.response.status, JSON.stringify(audits.error)).toBe(200);
    expect(Array.isArray(audits.data?.audits)).toBe(true);

    const cancel = await api.POST('/v1/media/jobs/{media_job_public_id}/cancel', { params });
    expect([204, 409], JSON.stringify(cancel.error)).toContain(cancel.response.status);
    const retry = await api.POST('/v1/media/jobs/{media_job_public_id}/retry', { params });
    expect([204, 409], JSON.stringify(retry.error)).toContain(retry.response.status);
    const persisted = await api.GET('/v1/media/profiles/{media_profile_public_id}', {
      params: profileParams,
    });
    expect(persisted.response.status, JSON.stringify(persisted.error)).toBe(200);
    expect(persisted.data).toEqual(created.data);
    expect(await readFile(fixture.sourcePath)).toEqual(fixture.sourceBytes);
  });

  for (const automation of [
    { endpoint: '/v1/media/discovery/schedules', code: 'media_discovery_schedule_disabled' },
    { endpoint: '/v1/media/discovery/watchers', code: 'media_discovery_watcher_disabled' },
  ] as const) {
    test(`rejects disabled ${automation.endpoint} runs without admitting jobs`, async ({ api }) => {
      const fixture = await profileFixture();
      const created = await api.POST('/v1/media/profiles', { body: fixture.request });
      expect(created.response.status, JSON.stringify(created.error)).toBe(201);
      const profileId = created.data?.media_profile_public_id;
      if (!profileId) {
        throw new Error('Missing media profile public id');
      }
      const run = await api.POST(automation.endpoint, {
        body: { media_profile_public_id: profileId, source_paths: [fixture.sourcePath] },
      });
      expect(run.response.status, JSON.stringify(run.error)).toBe(400);
      expect(run.error?.context).toContainEqual({ name: 'error_code', value: automation.code });
      const jobs = await api.GET('/v1/media/jobs', {
        params: { query: { media_profile_public_id: profileId } },
      });
      expect(jobs.response.status, JSON.stringify(jobs.error)).toBe(200);
      expect(jobs.data?.jobs).toEqual([]);
      expect(await readFile(fixture.sourcePath)).toEqual(fixture.sourceBytes);
    });
  }
});
