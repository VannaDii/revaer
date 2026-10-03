import { test, expect } from '../../fixtures/media';
import { authHeaders } from '../../support/headers';
import { execFileSync, spawn } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { createInterface } from 'node:readline';
import path from 'node:path';
import { repoRoot } from '../../support/paths';

const createPrecondition = { header: { 'If-None-Match': '*' } } as const;
const { load: loadYaml, dump: dumpYaml } = require('js-yaml') as {
  load: (text: string) => unknown; dump: (value: unknown) => string;
};

test.describe('Media profile creation', () => {
  test('persists a dry-run profile and rejects duplicate creation without mutation', async ({ api, profileFixture: fixture }) => {
    const created = await api.POST('/v1/media/profiles', {
      params: createPrecondition,
      body: fixture.request,
    });
    expect(created.response.status, JSON.stringify(created.error)).toBe(201);
    expect(created.data).toMatchObject({
      ...fixture.request,
    });
    expect(created.data?.latest_version).toBe(1);
    const profileId = created.data?.media_profile_public_id;
    if (!profileId) {
      throw new Error('Missing media profile public id');
    }
    const persisted = await api.GET('/v1/media/profiles/{media_profile_public_id}', {
      params: { path: { media_profile_public_id: profileId } },
    });
    expect(persisted.response.status, JSON.stringify(persisted.error)).toBe(200);
    expect(persisted.data).toEqual(created.data);
    const duplicate = await api.POST('/v1/media/profiles', {
      params: createPrecondition,
      body: { ...fixture.request, description: 'Duplicate must not persist' },
    });
    expect(duplicate.response.status, JSON.stringify(duplicate.error)).toBe(409);
    expect(duplicate.error?.context).toContainEqual({
      name: 'error_code', value: 'media_profile_key_conflict',
    });
    const afterConflict = await api.GET('/v1/media/profiles/{media_profile_public_id}', {
      params: { path: { media_profile_public_id: profileId } },
    });
    expect(afterConflict.response.status, JSON.stringify(afterConflict.error)).toBe(200);
    expect(afterConflict.data).toEqual(persisted.data);
    expect(fixture.readSource()).toEqual(fixture.sourceBytes);
  });

  for (const automation of ['schedule', 'watcher'] as const) {
    test(`rejects ${automation} fields in a profile without persisting a parent`, async ({ api, profileFixture: fixture }) => {
      const rejected = await api.POST('/v1/media/profiles', {
        params: createPrecondition,
        body: {
          ...fixture.request,
          ...(automation === 'schedule'
            ? { schedule_enabled: true, schedule_interval_minutes: 60 }
            : { watcher_enabled: true }),
        },
      });
      expect(rejected.response.status, JSON.stringify(rejected.error)).toBe(400);
      expect(rejected.error?.context).toContainEqual({ name: 'error_code', value: 'media_configuration_invalid' });
      const profiles = await api.GET('/v1/media/profiles');
      expect(profiles.response.status, JSON.stringify(profiles.error)).toBe(200);
      expect(
        profiles.data?.profiles.some((profile) => profile.profile_key === fixture.request.profile_key)
      ).toBe(false);
      expect(fixture.readSource()).toEqual(fixture.sourceBytes);
    });
  }
});

test.describe('Media profile updates', () => {
  test('requires explicit import authority without changing persisted profiles', async ({ api, baseUrl, session, profileFixture: fixture }) => {
    const created = await api.POST('/v1/media/profiles', { params: createPrecondition, body: fixture.request });
    expect(created.response.status, JSON.stringify(created.error)).toBe(201);
    const profileId = created.data?.media_profile_public_id;
    if (!profileId) throw new Error('Missing profile id');
    const exported = await api.GET('/v1/media/export');
    expect(exported.response.status, JSON.stringify(exported.error)).toBe(200);
    if (!exported.data) throw new Error('Missing native export');
    const yaml_payload = exported.data.yaml_payload;
    const missing = await api.POST('/v1/media/imports/apply', { body: { yaml_payload } });
    expect(missing.response.status, JSON.stringify(missing.error)).toBe(428);
    const incomplete = await api.POST('/v1/media/imports/apply', { body: { yaml_payload, preconditions: [] } });
    expect(incomplete.response.status, JSON.stringify(incomplete.error)).toBe(428);
    const row = { intent: 'match', kind: 'profiles', key: fixture.request.profile_key, expected_version: 1 };
    for (const preconditions of [null, [row, row], [{ ...row, expected_version: 0 }],
      [{ ...row, key: '/physical/path' }], [{ ...row, kind: 'unknown' }],
      [{ ...row, intent: 'create' }]]) {
      const rejected = await fetch(`${baseUrl}/v1/media/imports/apply`, {
        method: 'POST', headers: { ...authHeaders(session), 'Content-Type': 'application/json' },
        body: JSON.stringify({ yaml_payload, preconditions }),
      });
      expect(rejected.status).toBe(400);
    }
    const overflow = await fetch(`${baseUrl}/v1/media/imports/apply`, {
      method: 'POST', headers: { ...authHeaders(session), 'Content-Type': 'application/json' },
      body: JSON.stringify({ yaml_payload, preconditions: Array(129).fill(row) }),
    });
    expect(overflow.status).toBe(413);
    const persisted = await api.GET('/v1/media/profiles/{media_profile_public_id}', {
      params: { path: { media_profile_public_id: profileId } },
    });
    expect(persisted.data).toEqual(created.data);
    expect(fixture.readSource()).toEqual(fixture.sourceBytes);
  });

  test('locks the source before resource parents during complete replacement', async ({ api, mediaService, profileFixture: fixture }) => {
    const created = await api.POST('/v1/media/profiles', { params: createPrecondition, body: fixture.request });
    expect(created.response.status, JSON.stringify(created.error)).toBe(201);
    const profileId = created.data?.media_profile_public_id;
    if (!profileId) throw new Error('Missing profile id');
    const sql = readFileSync(path.join(repoRoot(), 'scripts/tests/database-profile-write-lock-fixture.sql'), 'utf8');
    const command = ['exec', '-i', mediaService.container.slice(0, -4), 'psql', '-X', '-qAt',
      '-U', 'postgres', '-d', mediaService.database, '-v', 'ON_ERROR_STOP=1'];
    const holder = spawn('docker', [...command, '-v', 'hold_source=true'], { stdio: ['pipe', 'pipe', 'pipe'] });
    const lines = createInterface({ input: holder.stdout });
    let ready = false;
    let diagnostics = '';
    let failure: Error | undefined;
    holder.once('error', error => { failure = error; });
    holder.stderr.on('data', chunk => { diagnostics = (diagnostics + chunk.toString()).slice(-2000); });
    lines.on('line', line => { if (line === 'LOCK_READY') ready = true; });
    const stopped = new Promise<number | null>(resolve => holder.once('close', resolve));
    holder.stdin.write(sql);
    let writing: Promise<{ status: number; latestVersion: number | undefined; error: unknown }> | undefined;
    try {
      await expect.poll(() => {
        if (failure) throw failure;
        if (holder.exitCode !== null) throw new Error(`Source lock holder exited: ${diagnostics}`);
        return ready;
      }, { timeout: 10000 }).toBe(true);
      writing = api.PUT('/v1/media/profiles/{media_profile_public_id}', {
        params: { path: { media_profile_public_id: profileId },
          header: { 'If-Match': `"media-profile:${profileId}:v1"` } },
        body: { ...fixture.request, description: 'Source lock regression' },
      }).then(result => ({ status: result.response.status,
        latestVersion: result.data?.latest_version, error: result.error }));
      await expect.poll(() => execFileSync('docker', [...command, '-v', 'hold_source=false'],
        { input: sql, encoding: 'utf8', timeout: 5000 }).trim(), { timeout: 10000 }).toBe('1|0');
    } finally {
      holder.stdin.end('ROLLBACK;\n');
      await expect.poll(() => holder.exitCode, { timeout: 10000 }).not.toBeNull();
      expect(await stopped, diagnostics).toBe(0);
      lines.close();
      if (writing) {
        const replaced = await writing;
        expect(replaced.status, JSON.stringify(replaced.error)).toBe(200);
        expect(replaced.latestVersion).toBe(2);
      }
    }
    expect(fixture.readSource()).toEqual(fixture.sourceBytes);
  });

  test('compares import authority with locked database heads without writes', async ({ api, profileFixture: fixture }) => {
    const created = await api.POST('/v1/media/profiles', { params: createPrecondition, body: fixture.request });
    expect(created.response.status, JSON.stringify(created.error)).toBe(201);
    const profileId = created.data?.media_profile_public_id;
    if (!profileId) throw new Error('Missing profile id');
    const yaml_payload = dumpYaml({ format_version: 1, kind: 'revaer.media.profile_bundle',
      metadata: { name: 'Owned native import fence fixture' }, profiles: [{ version: 1, ...fixture.request }] });
    for (const [precondition, code] of [
      [{ intent: 'match' as const, kind: 'profiles' as const, key: fixture.request.profile_key, expected_version: 2 }, 'media_configuration_version_conflict'],
      [{ intent: 'create' as const, kind: 'profiles' as const, key: fixture.request.profile_key }, 'media_configuration_create_conflict'],
    ] as const) {
      const rejected = await api.POST('/v1/media/imports/apply', { body: { yaml_payload, preconditions: [precondition] } });
      expect(rejected.response.status, JSON.stringify(rejected.error)).toBe(409);
      expect(rejected.error?.context).toContainEqual({ name: 'error_code', value: code });
    }
    const newKey = `${fixture.prefix}-import`;
    const newPayload = dumpYaml({ format_version: 1, kind: 'revaer.media.profile_bundle',
      metadata: { name: 'Owned new-key fixture' }, profiles: [{ version: 1, ...fixture.request, profile_key: newKey }] });
    const missing = await api.POST('/v1/media/imports/apply', { body: { yaml_payload: newPayload,
      preconditions: [{ intent: 'match', kind: 'profiles', key: newKey, expected_version: 1 }] } });
    expect(missing.response.status, JSON.stringify(missing.error)).toBe(409);
    expect(missing.error?.context).toContainEqual({ name: 'error_code', value: 'media_configuration_version_conflict' });
    const current = await api.POST('/v1/media/imports/apply', { body: { yaml_payload,
      preconditions: [{ intent: 'match', kind: 'profiles', key: fixture.request.profile_key, expected_version: 1 }] } });
    expect(current.response.status, JSON.stringify(current.error)).toBe(201);
    expect(current.data).toEqual({ forced_dry_run: true, media_profile_public_ids: [profileId],
      media_profile_import_draft_public_ids: [] });
    const persisted = await api.GET('/v1/media/profiles/{media_profile_public_id}', {
      params: { path: { media_profile_public_id: profileId } },
    });
    expect(persisted.data).toEqual(created.data);
    const profiles = await api.GET('/v1/media/profiles');
    expect(profiles.data?.profiles.some(row => row.profile_key === newKey)).toBe(false);
    expect(fixture.readSource()).toEqual(fixture.sourceBytes);
  });

  test('imports an unresolved native version as a dry-run draft without replacing the active head', async ({ api, profileFixture: fixture }) => {
    const created = await api.POST('/v1/media/profiles', { params: createPrecondition, body: fixture.request });
    expect(created.response.status, JSON.stringify(created.error)).toBe(201);
    const profileId = created.data?.media_profile_public_id;
    if (!profileId) throw new Error('Missing native profile');
    const payload = { format_version: 1, kind: 'revaer.media.profile_bundle',
      metadata: { name: 'Owned disabled draft fixture' }, profiles: [{ version: 2, ...fixture.request,
        dry_run_only: false, output_root_key: 'unmapped-output', workspace_root_key: 'unmapped-workspace' }] };
    const applied = await api.POST('/v1/media/imports/apply', { body: {
      yaml_payload: dumpYaml(payload), preconditions: [{ intent: 'match', kind: 'profiles',
        key: fixture.request.profile_key, expected_version: 1 }],
    } });
    expect(applied.response.status, JSON.stringify(applied.error)).toBe(201);
    expect(applied.data).toEqual({ forced_dry_run: true, media_profile_public_ids: [],
      media_profile_import_draft_public_ids: [profileId] });
    const persisted = await api.GET('/v1/media/profiles/{media_profile_public_id}', {
      params: { path: { media_profile_public_id: profileId } },
    });
    expect(persisted.data).toMatchObject({ latest_version: 2, active_version: 1,
      lifecycle_state: 'draft', enabled: false, dry_run_only: true,
      output_root_key: 'unmapped-output', workspace_root_key: 'unmapped-workspace' });
    expect(fixture.readSource()).toEqual(fixture.sourceBytes);
  });

  test('atomically imports native profiles and exact association pins and rolls back a late overlap', async ({ api, profileFixture: fixture }) => {
    const key = `${fixture.prefix}-native`;
    const associationKey = `${fixture.prefix}-native-assoc`;
    const bundle = { format_version: 1, kind: 'revaer.media.profile_bundle',
      metadata: { name: 'Owned atomic native fixture' },
      profiles: [{ ...fixture.request, profile_key: key, version: 3, dry_run_only: false }],
      discovery_associations: [{ association_key: associationKey, profile_key: key,
        profile_version: 3, source_root_key: 'source', root_relative_path: fixture.prefix,
        manual_enabled: true, watcher_enabled: false, schedule_enabled: false }] };
    const imported = await api.POST('/v1/media/imports/apply', { body: { yaml_payload: dumpYaml(bundle),
      preconditions: [{ intent: 'create', kind: 'profiles', key },
        { intent: 'create', kind: 'discovery_associations', key: associationKey }] } });
    expect(imported.response.status, JSON.stringify(imported.error)).toBe(201);
    expect(imported.data?.forced_dry_run).toBe(true);
    const profileId = imported.data?.media_profile_public_ids[0];
    if (!profileId) throw new Error('Missing imported profile');
    const profile = await api.GET('/v1/media/profiles/{media_profile_public_id}', {
      params: { path: { media_profile_public_id: profileId } },
    });
    expect(profile.data).toMatchObject({ latest_version: 3, active_version: 3, dry_run_only: true });
    const before = await api.GET('/v1/media/export');
    const repeated = await api.POST('/v1/media/imports/apply', { body: { yaml_payload: dumpYaml(bundle),
      preconditions: [{ intent: 'match', kind: 'profiles', key, expected_version: 3 },
        { intent: 'match', kind: 'discovery_associations', key: associationKey, expected_version: 1 }] } });
    expect(repeated.response.status, JSON.stringify(repeated.error)).toBe(201);
    const unchanged = await api.GET('/v1/media/export');
    expect(unchanged.data?.yaml_payload).toEqual(before.data?.yaml_payload);
    if (!unchanged.data) throw new Error('Missing association export');
    const persisted = loadYaml(unchanged.data.yaml_payload) as { discovery_associations: unknown[] };
    expect(persisted.discovery_associations).toContainEqual(bundle.discovery_associations[0]);
    const rejectedKey = `${fixture.prefix}-rejected`;
    const rejectedAssociation = `${fixture.prefix}-rejected-assoc`;
    const failedBundle = { ...bundle, profiles: [{ ...bundle.profiles[0], profile_key: rejectedKey, version: 1 }],
      discovery_associations: [{ ...bundle.discovery_associations[0], association_key: rejectedAssociation,
        profile_key: rejectedKey, profile_version: 1 }] };
    const failed = await api.POST('/v1/media/imports/apply', { body: { yaml_payload: dumpYaml(failedBundle),
      preconditions: [{ intent: 'create', kind: 'profiles', key: rejectedKey },
        { intent: 'create', kind: 'discovery_associations', key: rejectedAssociation }] } });
    expect(failed.response.status, JSON.stringify(failed.error)).toBe(409);
    expect(failed.error?.context).toContainEqual({ name: 'error_code', value: 'media_configuration_overlap' });
    const after = await api.GET('/v1/media/export');
    expect(after.data?.yaml_payload).toEqual(before.data?.yaml_payload);
    const profiles = await api.GET('/v1/media/profiles');
    expect(profiles.data?.profiles.some(row => row.profile_key === rejectedKey)).toBe(false);
    expect(fixture.readSource()).toEqual(fixture.sourceBytes);
  });

  test('rejects changed immutable catalog bodies with current import authority', async ({ api, profileFixture: fixture }) => {
    const created = await api.POST('/v1/media/profiles', { params: createPrecondition, body: fixture.request });
    expect(created.response.status, JSON.stringify(created.error)).toBe(201);
    const exported = await api.GET('/v1/media/export');
    expect(exported.response.status, JSON.stringify(exported.error)).toBe(200);
    if (!exported.data) throw new Error('Missing native export');
    const bundle = loadYaml(exported.data.yaml_payload) as Record<
      'targets' | 'policies' | 'compatibility_targets', Array<Record<string, unknown>>>;
    for (const [kind, field, ownedKey] of [
      ['targets', 'target_key', fixture.request.desired_target_key],
      ['policies', 'policy_key', fixture.request.policy_key],
      ['compatibility_targets', 'compatibility_target_key', undefined],
    ] as const) {
      const row = ownedKey ? bundle[kind].find(item => item[field] === ownedKey) : bundle[kind][0];
      if (!row || typeof row[field] !== 'string' || typeof row.version !== 'number') {
        throw new Error(`Missing catalog fixture: ${kind}`);
      }
      const key = row[field];
      const expected_version = Math.max(...bundle[kind].filter(item => item[field] === key)
        .map(item => Number(item.version)));
      const payload = (body: Record<string, unknown>) => dumpYaml({ format_version: 1,
        kind: 'revaer.media.profile_bundle', metadata: { name: 'Owned immutable catalog fixture' },
        [kind]: [body] });
      const preconditions = [{ intent: 'match' as const, kind, key, expected_version }];
      const repeated = await api.POST('/v1/media/imports/apply', {
        body: { yaml_payload: payload(row), preconditions },
      });
      expect(repeated.response.status, JSON.stringify(repeated.error)).toBe(201);
      const before = await api.GET('/v1/media/export');
      const rejected = await api.POST('/v1/media/imports/apply', {
        body: { yaml_payload: payload({ ...row, display_name: 'Changed immutable body' }), preconditions },
      });
      expect(rejected.response.status, JSON.stringify(rejected.error)).toBe(409);
      expect(rejected.error?.context).toContainEqual({ name: 'error_code', value: 'media_configuration_version_conflict' });
      const after = await api.GET('/v1/media/export');
      expect(after.data?.yaml_payload).toEqual(before.data?.yaml_payload);
      const newKey = `${fixture.prefix}-${kind === 'compatibility_targets' ? 'compat' : kind}`;
      const copied = await api.POST('/v1/media/imports/apply', { body: {
        yaml_payload: payload({ ...row, [field]: newKey }),
        preconditions: [{ intent: 'create', kind, key: newKey }],
      } });
      expect(copied.response.status, JSON.stringify(copied.error)).toBe(201);
      const updated = await api.GET('/v1/media/export');
      if (!updated.data) throw new Error('Missing updated catalog export');
      const persisted = loadYaml(updated.data.yaml_payload) as typeof bundle;
      expect(persisted[kind].find(item => item[field] === newKey)).toEqual({ ...row, [field]: newKey });
    }
    expect(fixture.readSource()).toEqual(fixture.sourceBytes);
  });

  test('bounds YAML transport and rejects malformed documents without writes', async ({ api, baseUrl, session, profileFixture: fixture }) => {
    const created = await api.POST('/v1/media/profiles', { params: createPrecondition, body: fixture.request });
    expect(created.response.status, JSON.stringify(created.error)).toBe(201);
    const profileId = created.data?.media_profile_public_id;
    if (!profileId) throw new Error('Missing profile id');
    const header = 'format_version: 1\nkind: revaer.media.profile_bundle\nmetadata: {name: Native}\n';
    const boundary = header.padEnd(4 * 1024 * 1024, ' ');
    const accepted = await api.POST('/v1/media/imports/validate', { body: { yaml_payload: boundary } });
    expect(accepted.response.status, JSON.stringify(accepted.error)).toBe(200);
    expect(accepted.data?.valid).toBe(true);
    for (const route of ['/v1/media/imports/validate', '/v1/media/imports/apply'] as const) {
      const oversized = await api.POST(route, { body: { yaml_payload: `${boundary} ` } });
      expect(oversized.response.status, JSON.stringify(oversized.error)).toBe(413);
      expect(oversized.error?.context).toContainEqual({ name: 'error_code', value: 'media_configuration_bound_exceeded' });
      for (const body of [
        { yaml_payload: header, unknown: true },
        { yaml_payload: `${header}metadata: {name: Duplicate}\n` },
        { yaml_payload: `${header}targets: &targets []\npolicies: *targets\n` },
        { yaml_payload: `${header}---\n${header}` },
        { yaml_payload: header.replace('{name: Native}', '!private {name: Native}') },
      ]) {
        const rejected = await fetch(`${baseUrl}${route}`, {
          method: 'POST', headers: { ...authHeaders(session), 'Content-Type': 'application/json' },
          body: JSON.stringify({ ...body, ...(route.endsWith('/apply') ? { preconditions: [] } : {}) }),
        });
        expect(rejected.status).toBe(400);
      }
    }
    const persisted = await api.GET('/v1/media/profiles/{media_profile_public_id}', {
      params: { path: { media_profile_public_id: profileId } },
    });
    expect(persisted.response.status, JSON.stringify(persisted.error)).toBe(200);
    expect(persisted.data).toEqual(created.data);
    expect(fixture.readSource()).toEqual(fixture.sourceBytes);
  });

  test('exports exact native heads and association pins without host authority', async ({ api, profileFixture: fixture }) => {
    const created = await api.POST('/v1/media/profiles', { params: createPrecondition, body: fixture.request });
    expect(created.response.status, JSON.stringify(created.error)).toBe(201);
    const profileId = created.data?.media_profile_public_id;
    if (!profileId) {
      throw new Error('Missing media profile public id');
    }
    const association = {
      association_key: `${fixture.request.profile_key}-export`,
      media_profile_public_id: profileId,
      profile_version: 1,
      source_root_key: 'source',
      root_relative_path: fixture.prefix,
      manual_enabled: false, watcher_enabled: false, schedule_enabled: false,
    };
    const saved = await api.POST('/v1/media/discovery-associations', {
      params: createPrecondition, body: association,
    });
    expect(saved.response.status, JSON.stringify(saved.error)).toBe(201);
    const nextBody = { ...fixture.request, description: 'New head retains earlier association pin' };
    const next = await api.PUT('/v1/media/profiles/{media_profile_public_id}', {
      params: {
        path: { media_profile_public_id: profileId },
        header: { 'If-Match': `"media-profile:${profileId}:v1"` },
      },
      body: nextBody,
    });
    expect(next.response.status, JSON.stringify(next.error)).toBe(200);
    expect(next.data?.latest_version).toBe(2);
    const exported = await api.GET('/v1/media/export');
    expect(exported.response.status, JSON.stringify(exported.error)).toBe(200);
    const payload = exported.data?.yaml_payload;
    if (!payload) {
      throw new Error('Missing portable YAML payload');
    }
    const bundle = loadYaml(payload) as {
      format_version: number; kind: string;
      profiles: (typeof fixture.request & { version: number })[];
      discovery_associations: Record<string, unknown>[];
    };
    expect(bundle.format_version).toBe(1);
    expect(bundle.kind).toBe('revaer.media.profile_bundle');
    expect(bundle.profiles.filter((profile) => profile.profile_key === fixture.request.profile_key)).toEqual([
      { version: 1, ...fixture.request }, { version: 2, ...nextBody },
    ]);
    expect(bundle.discovery_associations.filter((row) => row.association_key === association.association_key)).toEqual([
      { association_key: association.association_key, profile_key: fixture.request.profile_key,
        profile_version: 1, source_root_key: 'source', root_relative_path: fixture.prefix,
        manual_enabled: false, watcher_enabled: false, schedule_enabled: false },
    ]);
    for (const forbidden of ['/proof/', profileId, 'source_root:', 'output_root:', 'attestation', 'generation_id']) {
      expect(payload).not.toContain(forbidden);
    }
    const repeated = await api.GET('/v1/media/export');
    expect(repeated.response.status, JSON.stringify(repeated.error)).toBe(200);
    expect(repeated.data?.yaml_payload).toBe(payload);
    const validated = await api.POST('/v1/media/imports/validate', { body: { yaml_payload: payload } });
    expect(validated.response.status, JSON.stringify(validated.error)).toBe(200);
    expect(validated.data?.valid).toBe(true);
    expect(validated.data?.profile_count).toBe(bundle.profiles.length);
    const invalidPayload = dumpYaml({ ...bundle,
      profiles: [{ version: 1, ...fixture.request,
        desired_target_key: 'missing-target', policy_key: 'missing-policy' }],
      discovery_associations: [],
    });
    const invalid = await api.POST('/v1/media/imports/validate', { body: { yaml_payload: invalidPayload } });
    expect(invalid.response.status, JSON.stringify(invalid.error)).toBe(200);
    expect(invalid.data?.valid).toBe(false);
    expect(invalid.data?.issues).toEqual(expect.arrayContaining([
      { code: 'media_yaml_desired_target_not_found', pointer: '/profiles/0/desired_target_key', blocking: true },
      { code: 'media_yaml_policy_profile_not_found', pointer: '/profiles/0/policy_key', blocking: true },
    ]));
    const rejectedApply = await api.POST('/v1/media/imports/apply', { body: { yaml_payload: invalidPayload, preconditions: [] } });
    expect(rejectedApply.response.status, JSON.stringify(rejectedApply.error)).toBe(400);
    const persisted = await api.GET('/v1/media/profiles/{media_profile_public_id}', {
      params: { path: { media_profile_public_id: profileId } },
    });
    expect(persisted.response.status, JSON.stringify(persisted.error)).toBe(200);
    expect(persisted.data).toEqual(next.data);
    expect(fixture.readSource()).toEqual(fixture.sourceBytes);
  });

  test('rejects retired target pin and clear writes without changing the native profile', async ({ api, baseUrl, session, profileFixture: fixture }) => {
    const created = await api.POST('/v1/media/profiles', { params: createPrecondition, body: fixture.request });
    expect(created.response.status, JSON.stringify(created.error)).toBe(201);
    const profileId = created.data?.media_profile_public_id;
    if (!profileId) {
      throw new Error('Missing media profile public id');
    }
    const params = { path: { media_profile_public_id: profileId } };
    const original = await api.GET('/v1/media/profiles/{media_profile_public_id}', { params });
    expect(original.response.status, JSON.stringify(original.error)).toBe(200);
    for (const body of [
      { target_key: fixture.request.desired_target_key, version: fixture.request.desired_target_version },
      {},
    ]) {
      const rejected = await fetch(`${baseUrl}/v1/media/profiles/${profileId}/desired-target`, {
        method: 'PATCH',
        headers: { ...authHeaders(session), 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });
      expect(rejected.status).toBe(404);
      const persisted = await api.GET('/v1/media/profiles/{media_profile_public_id}', { params });
      expect(persisted.response.status, JSON.stringify(persisted.error)).toBe(200);
      expect(persisted.data).toEqual(original.data);
      expect(persisted.response.headers.get('etag')).toBe(original.response.headers.get('etag'));
      expect(fixture.readSource()).toEqual(fixture.sourceBytes);
    }
    const replaced = await api.PUT('/v1/media/profiles/{media_profile_public_id}', {
      params: { ...params, header: { 'If-Match': `"media-profile:${profileId}:v1"` } },
      body: { ...fixture.request, description: 'Complete replacement remains available' },
    });
    expect(replaced.response.status, JSON.stringify(replaced.error)).toBe(200);
    expect(replaced.data?.latest_version).toBe(2);
    expect(replaced.data?.desired_target_key).toBe(fixture.request.desired_target_key);
    expect(replaced.data?.desired_target_version).toBe(fixture.request.desired_target_version);
    expect(fixture.readSource()).toEqual(fixture.sourceBytes);
  });

  test('replaces metadata without changing the logical roots or enabling automation', async ({ api, profileFixture: fixture }) => {
    const created = await api.POST('/v1/media/profiles', { params: createPrecondition, body: fixture.request });
    expect(created.response.status, JSON.stringify(created.error)).toBe(201);
    const profileId = created.data?.media_profile_public_id;
    if (!profileId) {
      throw new Error('Missing media profile public id');
    }
    const params = { path: { media_profile_public_id: profileId } };
    const body = { ...fixture.request, description: 'Updated metadata' };
    const patched = await api.PUT('/v1/media/profiles/{media_profile_public_id}', {
      params: { ...params, header: { 'If-Match': `"media-profile:${profileId}:v1"` } },
      body,
    });
    expect(patched.response.status, JSON.stringify(patched.error)).toBe(200);
    expect(patched.data).toMatchObject(body);
    expect(patched.data?.latest_version).toBe(2);
    const persisted = await api.GET('/v1/media/profiles/{media_profile_public_id}', { params });
    expect(persisted.response.status, JSON.stringify(persisted.error)).toBe(200);
    expect(persisted.data).toEqual(patched.data);
    expect(fixture.readSource()).toEqual(fixture.sourceBytes);
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
    test(`rejects ${update.name} in a complete profile replacement without mutation`, async ({ api, profileFixture: fixture }) => {
      const created = await api.POST('/v1/media/profiles', { params: createPrecondition, body: fixture.request });
      expect(created.response.status, JSON.stringify(created.error)).toBe(201);
      const profileId = created.data?.media_profile_public_id;
      if (!profileId) {
        throw new Error('Missing media profile public id');
      }
      const params = { path: { media_profile_public_id: profileId } };
      const rejected = await api.PUT('/v1/media/profiles/{media_profile_public_id}', {
        params: { ...params, header: { 'If-Match': `"media-profile:${profileId}:v1"` } },
        body: { ...fixture.request, ...update.body },
      });
      expect(rejected.response.status, JSON.stringify(rejected.error)).toBe(400);
      expect(rejected.error?.context).toContainEqual({ name: 'error_code', value: 'media_configuration_invalid' });
      const persisted = await api.GET('/v1/media/profiles/{media_profile_public_id}', { params });
      expect(persisted.response.status, JSON.stringify(persisted.error)).toBe(200);
      expect(persisted.data).toEqual(created.data);
      expect(fixture.readSource()).toEqual(fixture.sourceBytes);
    });
  }
});

test.describe('Dry-run profile discovery', () => {
  test('admits an association-scoped manual job and reads its diagnostics without enabling automation', async ({ api, profileFixture: fixture }) => {
    const created = await api.POST('/v1/media/profiles', { params: createPrecondition, body: fixture.request });
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
    expect(readiness.data?.profile).toEqual(created.data);
    expect(readiness.data?.active_profile).toEqual(fixture.request);
    expect(readiness.data?.binding_ready).toBe(true);
    expect(readiness.data?.destructive_ready).toBe(true);
    expect(readiness.data?.active_association_count).toBe(0);

    const association = await api.POST('/v1/media/discovery-associations', {
      params: createPrecondition,
      body: { association_key: fixture.prefix, media_profile_public_id: profileId, profile_version: 1,
        source_root_key: 'source', root_relative_path: fixture.prefix, manual_enabled: true,
        watcher_enabled: false, schedule_enabled: false },
    });
    expect(association.response.status, JSON.stringify(association.error)).toBe(201);
    const associationId = association.data?.media_discovery_association_public_id;
    if (!associationId) throw new Error('Missing discovery association identity');
    const associatedReadiness = await api.GET('/v1/media/profiles/{media_profile_public_id}/readiness', {
      params: profileParams,
    });
    expect(associatedReadiness.response.status, JSON.stringify(associatedReadiness.error)).toBe(200);
    expect(associatedReadiness.data?.active_association_count).toBe(1);
    const planning = await api.POST('/v1/media/planning/preview', {
      body: { media_discovery_association_public_id: associationId, source_path: fixture.relativePath },
    });
    expect(planning.response.status, JSON.stringify(planning.error)).toBe(200);
    expect(planning.data?.accepted).toBe(true);
    const body = { media_discovery_association_public_id: associationId, source_paths: [fixture.relativePath] };
    const preview = await api.POST('/v1/media/discovery/preview', { body });
    expect(preview.response.status, JSON.stringify(preview.error)).toBe(200);
    expect(preview.data?.previews[0]?.accepted).toBe(true);
    const run = await api.POST('/v1/media/discovery/runs', { body });
    expect(run.response.status, JSON.stringify(run.error)).toBe(201);
    expect(run.data?.skipped).toEqual([]);
    expect(run.data?.queued_jobs).toHaveLength(1);
    expect(run.data?.queued_jobs[0]).toMatchObject({
      source_path: fixture.relativePath,
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
      { source_path: fixture.relativePath, reason: 'media_discovery_source_unchanged' },
    ]);

    const jobs = await api.GET('/v1/media/jobs', {
      params: { query: { media_profile_public_id: profileId } },
    });
    expect(jobs.response.status, JSON.stringify(jobs.error)).toBe(200);
    expect(jobs.data?.jobs.map((job) => job.media_job_public_id)).toEqual([jobId]);
    const params = { path: { media_job_public_id: jobId } };
    const job = await api.GET('/v1/media/jobs/{media_job_public_id}', { params });
    expect(job.response.status, JSON.stringify(job.error)).toBe(200);
    expect(job.data?.source_path).toBe(fixture.relativePath);
    expect(job.data?.output_path).toBe(fixture.relativePath);
    expect(JSON.stringify({ jobs: jobs.data, job: job.data })).not.toContain('/proof/');

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
    expect(fixture.readSource()).toEqual(fixture.sourceBytes);
  });

  for (const automation of [
    { endpoint: '/v1/media/discovery/schedules', code: 'media_discovery_schedule_disabled' },
    { endpoint: '/v1/media/discovery/watchers', code: 'media_discovery_watcher_disabled' },
  ] as const) {
    test(`rejects disabled ${automation.endpoint} runs without admitting jobs`, async ({ api, baseUrl, session, profileFixture: fixture }) => {
      const created = await api.POST('/v1/media/profiles', { params: createPrecondition, body: fixture.request });
      expect(created.response.status, JSON.stringify(created.error)).toBe(201);
      const profileId = created.data?.media_profile_public_id;
      if (!profileId) {
        throw new Error('Missing media profile public id');
      }
      const association = await api.POST('/v1/media/discovery-associations', {
        params: createPrecondition,
        body: { association_key: fixture.prefix, media_profile_public_id: profileId, profile_version: 1,
          source_root_key: 'source', root_relative_path: fixture.prefix, manual_enabled: true,
          watcher_enabled: false, schedule_enabled: false },
      });
      expect(association.response.status, JSON.stringify(association.error)).toBe(201);
      const associationId = association.data?.media_discovery_association_public_id;
      if (!associationId) throw new Error('Missing discovery association identity');
      const schedules = await api.GET('/v1/media/discovery/schedules', { params: { query: { limit: 200 } } });
      expect(schedules.response.status, JSON.stringify(schedules.error)).toBe(200);
      expect(schedules.data?.schedules.find((row) => row.media_discovery_association_public_id === associationId))
        .toEqual(association.data);
      const watchers = await api.GET('/v1/media/discovery/watchers', { params: { query: { limit: 200 } } });
      expect(watchers.response.status, JSON.stringify(watchers.error)).toBe(200);
      expect(watchers.data?.watchers.find((row) => row.media_discovery_association_public_id === associationId))
        .toEqual(association.data);
      expect(JSON.stringify({ schedules: schedules.data, watchers: watchers.data })).not.toContain('/proof/');
      for (const route of ['/v1/media/discovery/schedules', '/v1/media/discovery/watchers'] as const) {
        const first = await api.GET(route, { params: { query: { limit: 1 } } });
        expect(first.response.status, JSON.stringify(first.error)).toBe(200);
        if (first.data?.next_cursor) {
          const next = await api.GET(route, { params: { query: { limit: 1, cursor: first.data.next_cursor } } });
          expect(next.response.status, JSON.stringify(next.error)).toBe(200);
          expect(next.data).not.toEqual(first.data);
        }
        for (const query of ['limit=0', 'limit=201', 'cursor=invalid', 'unexpected=value']) {
          const rejected = await fetch(new URL(`${route}?${query}`, baseUrl), { headers: authHeaders(session) });
          expect(rejected.status, await rejected.text()).toBe(400);
        }
      }
      const run = await api.POST(automation.endpoint, {
        body: { media_discovery_association_public_id: associationId, source_paths: [fixture.relativePath] },
      });
      expect(run.response.status, JSON.stringify(run.error)).toBe(400);
      expect(run.error?.context).toContainEqual({ name: 'error_code', value: automation.code });
      const missing = await api.POST(automation.endpoint, {
        body: { media_discovery_association_public_id: '00000000-0000-0000-0000-000000000001',
          source_paths: [fixture.relativePath] },
      });
      expect(missing.response.status, JSON.stringify(missing.error)).toBe(404);
      for (const body of [
        { media_profile_public_id: profileId, source_paths: [fixture.relativePath] },
        { media_discovery_association_public_id: associationId, source_paths: [fixture.sourcePath] },
        { media_discovery_association_public_id: associationId, source_paths: Array(129).fill(fixture.relativePath) },
        { media_discovery_association_public_id: associationId, source_paths: [] },
      ]) {
        const rejected = await fetch(new URL(automation.endpoint, baseUrl), {
          method: 'POST', headers: { ...authHeaders(session), 'Content-Type': 'application/json' },
          body: JSON.stringify(body),
        });
        expect(rejected.status, JSON.stringify(await rejected.json())).toBe(400);
      }
      const jobs = await api.GET('/v1/media/jobs', {
        params: { query: { media_profile_public_id: profileId } },
      });
      expect(jobs.response.status, JSON.stringify(jobs.error)).toBe(200);
      expect(jobs.data?.jobs).toEqual([]);
      expect(fixture.readSource()).toEqual(fixture.sourceBytes);
    });
  }
});
