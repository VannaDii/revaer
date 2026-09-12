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
          schedule_enabled: automation === 'schedule',
          schedule_interval_minutes: 60,
          watcher_enabled: automation === 'watcher',
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
