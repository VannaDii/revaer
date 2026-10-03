import { randomUUID } from 'node:crypto';
import { test, expect } from '../../fixtures/media';
import { apiFetchRaw } from '../../support/api/raw';
import { authHeaders } from '../../support/headers';

const MISSING_JOB_ID = '00000000-0000-0000-0000-000000000001';
const { load: loadYaml } = require('js-yaml') as { load: (text: string) => unknown };

function fixtureImportPreconditions(yaml: string) {
  const bundle = loadYaml(yaml) as Record<string, unknown>;
  const families = [
    ['compatibility_targets', 'compatibility_target_key'], ['targets', 'target_key'],
    ['policies', 'policy_key'], ['profiles', 'profile_key'],
    ['discovery_associations', 'association_key'],
  ] as const;
  type Kind = typeof families[number][0];
  const fences = new Map<string, { intent: 'match'; kind: Kind; key: string; expected_version: number }>();
  for (const [kind, keyField] of families) {
    const rows = bundle[kind];
    if (!Array.isArray(rows)) throw new Error('Missing native fixture resource array');
    for (const row of rows) {
      const key = row[keyField];
      // This owned fixture creates associations only; none has a replacement head.
      const version = kind === 'discovery_associations' ? 1 : row.version;
      if (typeof key !== 'string' || !Number.isInteger(version) || version <= 0) {
        throw new Error('Invalid native fixture resource identity');
      }
      const identity = JSON.stringify([kind, key]);
      const previous = fences.get(identity);
      fences.set(identity, { intent: 'match', kind, key,
        expected_version: Math.max(version, previous?.expected_version ?? version) });
    }
  }
  return [...fences.values()];
}

const ROUTED_OPERATIONS = [
  ['GET', '/v1/media/capabilities'],
  ['GET', '/v1/media/capabilities/readiness'],
  ['GET', '/v1/media/compatibility-targets'],
  ['GET', '/v1/media/compliance'],
  ['GET', '/v1/media/discovery/schedules'],
  ['GET', '/v1/media/discovery/watchers'],
  ['GET', '/v1/media/export'],
  ['GET', '/v1/media/job-retention'],
  ['GET', '/v1/media/jobs'],
  ['GET', '/v1/media/jobs/recent'],
  ['GET', '/v1/media/jobs/{media_job_public_id}'],
  ['GET', '/v1/media/jobs/{media_job_public_id}/artifacts'],
  ['GET', '/v1/media/jobs/{media_job_public_id}/compact-audits'],
  ['GET', '/v1/media/jobs/{media_job_public_id}/diagnostics'],
  ['GET', '/v1/media/jobs/{media_job_public_id}/operations'],
  ['GET', '/v1/media/jobs/{media_job_public_id}/plan-reasons'],
  ['GET', '/v1/media/jobs/{media_job_public_id}/verification-checks'],
  ['GET', '/v1/media/jobs/{media_job_public_id}/violations'],
  ['GET', '/v1/media/policies'],
  ['GET', '/v1/media/profiles'],
  ['GET', '/v1/media/profiles/{media_profile_public_id}'],
  ['GET', '/v1/media/targets'],
  ['PATCH', '/v1/media/job-retention'],
  ['PUT', '/v1/media/profiles/{media_profile_public_id}'],
  ['POST', '/v1/media/capabilities/refresh'],
  ['POST', '/v1/media/compatibility-targets'],
  ['POST', '/v1/media/discovery/preview'],
  ['POST', '/v1/media/discovery/runs'],
  ['POST', '/v1/media/discovery/schedules'],
  ['POST', '/v1/media/discovery/watchers'],
  ['POST', '/v1/media/imports/apply'],
  ['POST', '/v1/media/imports/validate'],
  ['POST', '/v1/media/jobs/{media_job_public_id}/cancel'],
  ['POST', '/v1/media/jobs/{media_job_public_id}/retry'],
  ['POST', '/v1/media/planning/preview'],
  ['POST', '/v1/media/policies'],
  ['POST', '/v1/media/profiles'],
  ['POST', '/v1/media/profiles/validate'],
  ['POST', '/v1/media/targets'],
] as const;

const RETIRED_WRITE_OPERATIONS = [
  ['POST', '/v1/media/jobs'],
  ['POST', '/v1/media/jobs/{media_job_public_id}/phases'],
  ['PATCH', '/v1/media/profiles/{media_profile_public_id}'],
] as const;

test.describe('Media API', () => {
  test('updates a compatibility target and preserves it after a rejected write', async ({ api }) => {
    const target = {
      compatibility_target_key: `target-write-${randomUUID()}`,
      version: 1,
      display_name: 'Original target',
      video_codec: 'hevc',
      audio_codec: 'aac',
      audio_channels: 2,
      audio_channel_layout: 'stereo',
      subtitle_policy: 'selected',
    };
    const created = await api.POST('/v1/media/compatibility-targets', { body: target });
    expect(created.response.status, JSON.stringify(created.error)).toBe(201);
    expect(created.data).toMatchObject(target);

    const updatedTarget = { ...target, display_name: 'Updated target' };
    const updated = await api.POST('/v1/media/compatibility-targets', { body: updatedTarget });
    expect(updated.response.status, JSON.stringify(updated.error)).toBe(201);
    expect(updated.data).toMatchObject(updatedTarget);

    const rejected = await api.POST('/v1/media/compatibility-targets', {
      body: { ...target, display_name: '' },
    });
    expect(rejected.response.status, JSON.stringify(rejected.error)).toBe(400);
    const persisted = await api.GET('/v1/media/compatibility-targets');
    expect(persisted.response.status, JSON.stringify(persisted.error)).toBe(200);
    expect(persisted.data?.targets.filter(
      (entry) => entry.compatibility_target_key === target.compatibility_target_key
    )).toEqual([updated.data]);
  });

  test('routes bounded empty requests without server errors', async ({ baseUrl, session }) => {
    for (const [method, route] of ROUTED_OPERATIONS) {
      const response = await apiFetchRaw({
        baseUrl,
        method,
        route,
        path: {
          media_job_public_id: MISSING_JOB_ID,
          media_profile_public_id: MISSING_JOB_ID,
        },
        headers: authHeaders(session),
      });
      expect(response.status, `${method} ${route} returned a server error`).toBeLessThan(500);
      expect(response.status, `${method} ${route} is not wired`).not.toBe(405);
    }
  });

  test('keeps retired and worker-owned media writes unavailable', async ({ baseUrl, session }) => {
    for (const [method, route] of RETIRED_WRITE_OPERATIONS) {
      const response = await apiFetchRaw({
        baseUrl,
        method,
        route,
        path: {
          media_job_public_id: MISSING_JOB_ID,
          media_profile_public_id: MISSING_JOB_ID,
        },
        headers: authHeaders(session),
      });
      expect(response.status, `${method} ${route} must remain unavailable`).toBe(405);
      if (method === 'PATCH') {
        expect(response.headers.get('allow')).toBe('GET, HEAD, PUT');
      }
    }
  });

  test('covers media profiles jobs capabilities manual discovery and diagnostics', async ({ api, profileFixture: fixture }) => {
    test.setTimeout(60_000);

    const suffix = fixture.prefix;
    const sourceRoot = '/proof/source';
    const outputRoot = '/proof/output';
    const sourcePath = fixture.relativePath;
    const manualPath = fixture.relativePath;

    const createdProfile = await api.POST('/v1/media/profiles', {
      params: { header: { 'If-None-Match': '*' } },
      body: fixture.request,
    });
    expect(createdProfile.response.status, JSON.stringify(createdProfile.error)).toBe(201);
    const profileId = createdProfile.data?.media_profile_public_id;
    if (!profileId) {
      throw new Error('Missing media profile public id');
    }

    const profiles = await api.GET('/v1/media/profiles');
    expect(profiles.response.status).toBe(200);
    expect(
      profiles.data?.profiles.some((profile) => profile.media_profile_public_id === profileId)
    ).toBeTruthy();

    const profile = await api.GET('/v1/media/profiles/{media_profile_public_id}', {
      params: { path: { media_profile_public_id: profileId } },
    });
    expect(profile.response.status).toBe(200);
    expect(profile.data).toEqual(createdProfile.data);
    const profileTag = profile.response.headers.get('etag');
    if (!profileTag) throw new Error('Missing profile version fence');

    const updatedBody = { ...fixture.request, description: 'Updated media profile metadata' };
    const patchedProfile = await api.PUT('/v1/media/profiles/{media_profile_public_id}', {
      params: { path: { media_profile_public_id: profileId }, header: { 'If-Match': profileTag } },
      body: updatedBody,
    });
    expect(patchedProfile.response.status, JSON.stringify(patchedProfile.error)).toBe(200);
    expect(patchedProfile.data).toMatchObject(updatedBody);
    expect(patchedProfile.data?.latest_version).toBe(2);
    expect(fixture.readSource()).toEqual(fixture.sourceBytes);

    const validatedProfile = await api.POST('/v1/media/profiles/validate', {
      body: {
        profile_key: `e2e-media-validated-${suffix}`,
        source_root: sourceRoot,
        output_root: outputRoot,
        dry_run_only: true,
        retention_days: 30,
        schedule_enabled: false,
        watcher_enabled: false,
      },
    });
    expect(validatedProfile.response.status).toBe(200);
    expect(validatedProfile.data?.valid).toBe(true);

    const targets = await api.GET('/v1/media/compatibility-targets');
    expect(targets.response.status).toBe(200);
    expect(
      targets.data?.targets.some((target) => target.compatibility_target_key === 'hevc-aac')
    ).toBeTruthy();

    const upsertedTarget = await api.POST('/v1/media/compatibility-targets', {
      body: {
        compatibility_target_key: `e2e-target-${suffix}`,
        version: 1,
        display_name: `E2E target ${suffix}`,
        video_codec: 'hevc',
        audio_codec: 'aac',
        audio_channels: 2,
        audio_channel_layout: 'stereo',
        subtitle_policy: 'selected',
      },
    });
    expect(upsertedTarget.response.status).toBe(201);
    expect(upsertedTarget.data?.compatibility_target_key).toBe(`e2e-target-${suffix}`);
    expect(upsertedTarget.data?.audio_channels).toBe(2);
    expect(upsertedTarget.data?.audio_channel_layout).toBe('stereo');

    const desiredTargetKey = `e2e-desired-target-${suffix}`;
    const createdDesiredTarget = await api.POST('/v1/media/targets', {
      body: {
        target_key: desiredTargetKey,
        version: 1,
        display_name: `E2E desired target ${suffix}`,
        container_format: 'matroska',
        streams: [
          {
            stream_key: 'video-main',
            stream_kind: 'video',
            optional: false,
            sort_order: 0,
            codec: 'hevc',
            default_disposition: true,
            forced_disposition: false,
          },
          {
            stream_key: 'audio-main',
            stream_kind: 'audio',
            semantic_role: 'primary',
            language_code: 'eng',
            optional: false,
            sort_order: 1,
            codec: 'aac',
            channel_count: 2,
            channel_layout: 'stereo',
            default_disposition: true,
            forced_disposition: false,
          },
          {
            stream_key: 'subtitle-full',
            stream_kind: 'subtitle',
            semantic_role: 'primary',
            language_code: 'eng',
            optional: true,
            sort_order: 2,
            codec: 'subrip',
            default_disposition: false,
            forced_disposition: false,
          },
        ],
      },
    });
    expect(createdDesiredTarget.response.status).toBe(201);
    expect(createdDesiredTarget.data?.target_key).toBe(desiredTargetKey);
    expect(createdDesiredTarget.data?.streams.map((stream) => stream.stream_key)).toEqual([
      'video-main',
      'audio-main',
      'subtitle-full',
    ]);

    const desiredTargets = await api.GET('/v1/media/targets');
    expect(desiredTargets.response.status).toBe(200);
    expect(
      desiredTargets.data?.targets.some((target) => target.target_key === desiredTargetKey)
    ).toBeTruthy();

    const updatedTag = patchedProfile.response.headers.get('etag');
    if (!updatedTag) throw new Error('Missing updated profile version fence');
    const pinnedDesiredTarget = await api.PUT(
      '/v1/media/profiles/{media_profile_public_id}',
      {
        params: { path: { media_profile_public_id: profileId }, header: { 'If-Match': updatedTag } },
        body: { ...updatedBody, desired_target_key: desiredTargetKey, desired_target_version: 1 },
      }
    );
    expect(pinnedDesiredTarget.response.status, JSON.stringify(pinnedDesiredTarget.error)).toBe(200);
    expect(pinnedDesiredTarget.data?.latest_version).toBe(3);

    const profileWithDesiredTarget = await api.GET(
      '/v1/media/profiles/{media_profile_public_id}',
      { params: { path: { media_profile_public_id: profileId } } }
    );
    expect(profileWithDesiredTarget.response.status).toBe(200);
    expect(profileWithDesiredTarget.data?.desired_target_key).toBe(desiredTargetKey);
    expect(profileWithDesiredTarget.data?.desired_target_version).toBe(1);

    const policies = await api.GET('/v1/media/policies');
    expect(policies.response.status).toBe(200);
    expect(policies.data?.policies.some((policy) => policy.policy_key === 'safe_dry_run')).toBe(
      true
    );

    const upsertedPolicy = await api.POST('/v1/media/policies', {
      body: {
        policy_key: `e2e-policy-${suffix}`,
        output: { dry_run: true, replacement_mode: 'disabled', quarantine_enabled: true,
          preserve_permissions: true, preserve_ownership: true },
        version: 1,
        display_name: `E2E policy ${suffix}`,
        video_intent: 'general',
        verification_strictness: 'strict',
        verification_duration_tolerance_millis: 100,
        verification_mux_validation: true,
        verification_decode_all_streams: true,
        verification_keyframe_seek: true,
        verification_playback_probe: true,
      },
    });
    expect(upsertedPolicy.response.status).toBe(201);
    expect(upsertedPolicy.data?.policy_key).toBe(`e2e-policy-${suffix}`);
    expect(upsertedPolicy.data?.verification_playback_probe).toBe(true);

    const invalidProfileValidation = await api.POST('/v1/media/profiles/validate', {
      body: {
        profile_key: `e2e-media-invalid-${suffix}`,
        source_root: `${sourceRoot}/invalid-validation`,
        output_root: `${outputRoot}/invalid-validation`,
        dry_run_only: true,
        retention_days: 30,
        compatibility_target_key: `missing-target-${suffix}`,
        policy_key: `missing-policy-${suffix}`,
        schedule_enabled: false,
        watcher_enabled: false,
      },
    });
    expect(invalidProfileValidation.response.status).toBe(200);
    expect(invalidProfileValidation.data?.valid).toBe(false);
    expect(invalidProfileValidation.data?.issues).toContain(
      'media_profile_compatibility_target_not_found'
    );
    expect(invalidProfileValidation.data?.issues).toContain('media_profile_policy_profile_not_found');

    const invalidProfileCreate = await api.POST('/v1/media/profiles', {
      params: { header: { 'If-None-Match': '*' } },
      body: {
        profile_key: `e2e-media-invalid-create-${suffix}`,
        source_root: `${sourceRoot}/invalid-create`,
        output_root: `${outputRoot}/invalid-create`,
        dry_run_only: true,
        retention_days: 30,
        compatibility_target_key: `missing-target-${suffix}`,
        policy_key: `missing-policy-${suffix}`,
        schedule_enabled: false,
        watcher_enabled: false,
      },
    });
    expect(invalidProfileCreate.response.status).toBe(400);

    const retention = await api.GET('/v1/media/job-retention');
    expect(retention.response.status).toBe(200);
    expect(retention.data?.completed_enabled).toBe(false);
    expect(retention.data?.completed_mode).toBe('age');
    expect(retention.data?.completed_limit).toBeGreaterThan(0);
    expect(retention.data?.failed_diagnostic_enabled).toBe(true);

    const patchedRetention = await api.PATCH('/v1/media/job-retention', {
      body: {
        completed_enabled: true,
        completed_mode: 'count',
        completed_limit: 32,
        failed_diagnostic_enabled: true,
        failed_diagnostic_mode: 'age',
        failed_diagnostic_limit: 33,
      },
    });
    expect(patchedRetention.response.status).toBe(200);
    expect(patchedRetention.data?.completed_enabled).toBe(true);
    expect(patchedRetention.data?.completed_mode).toBe('count');
    expect(patchedRetention.data?.completed_limit).toBe(32);
    expect(patchedRetention.data?.failed_diagnostic_enabled).toBe(true);
    expect(patchedRetention.data?.failed_diagnostic_mode).toBe('age');
    expect(patchedRetention.data?.failed_diagnostic_limit).toBe(33);

    const latestCapability = await api.GET('/v1/media/capabilities');
    expect(latestCapability.response.status).toBe(200);
    if (latestCapability.data?.snapshot) {
      expect(Array.isArray(latestCapability.data.snapshot.features)).toBe(true);
      expect(Array.isArray(latestCapability.data.snapshot.muxers)).toBe(true);
      expect(latestCapability.data.snapshot.license_mode.length).toBeGreaterThan(0);
    }

    const readiness = await api.GET('/v1/media/capabilities/readiness');
    expect(readiness.response.status).toBe(200);
    expect(typeof readiness.data?.ready).toBe('boolean');

    const profileReadiness = await api.GET(
      '/v1/media/profiles/{media_profile_public_id}/readiness',
      { params: { path: { media_profile_public_id: profileId } } }
    );
    expect(profileReadiness.response.status).toBe(200);
    expect(profileReadiness.data?.profile.media_profile_public_id).toBe(profileId);
    expect(profileReadiness.data?.binding_ready).toBe(true);
    expect(profileReadiness.data?.destructive_ready).toBe(true);
    expect(profileReadiness.data?.profile).toEqual(profileWithDesiredTarget.data);
    expect(fixture.readSource()).toEqual(fixture.sourceBytes);

    const refresh = await api.POST('/v1/media/capabilities/refresh');
    expect([201, 500, 503]).toContain(refresh.response.status);

    const compliance = await api.GET('/v1/media/compliance');
    expect(compliance.response.status).toBe(200);
    expect(compliance.data?.license_mode).toBe('redistributable-gplv3-runtime');

    const exported = await api.GET('/v1/media/export');
    expect(exported.response.status).toBe(200);
    const yamlPayload = exported.data?.yaml_payload;
    if (!yamlPayload) {
      throw new Error('Missing media YAML payload');
    }

    const validated = await api.POST('/v1/media/imports/validate', {
      body: { yaml_payload: yamlPayload },
    });
    expect(validated.response.status).toBe(200);
    expect(validated.data?.valid).toBe(true);

    const invalidYamlPayload = [
      'format_version: 1',
      'kind: revaer.media.profile_bundle',
      'metadata:',
      '  name: Invalid catalog references',
      'profiles:',
      `  - profile_key: e2e-yaml-invalid-${suffix}`,
      '    version: 1',
      '    display_name: Invalid references',
      "    description: ''",
      '    enabled: false',
      '    output_root_key: source',
      '    workspace_root_key: workspace',
      '    quarantine_root_key: quarantine',
      '    dry_run_only: true',
      `    desired_target_key: missing-target-${suffix}`,
      '    desired_target_version: 1',
      `    policy_key: missing-policy-${suffix}`,
      '    policy_version: 1',
    ].join('\n');
    const invalidYamlValidation = await api.POST('/v1/media/imports/validate', {
      body: { yaml_payload: invalidYamlPayload },
    });
    expect(invalidYamlValidation.response.status).toBe(200);
    expect(invalidYamlValidation.data?.valid).toBe(false);
    expect(
      invalidYamlValidation.data?.issues.some(
        (issue) =>
          issue.code === 'media_yaml_desired_target_not_found' &&
          issue.pointer === '/profiles/0/desired_target_key' &&
          issue.blocking
      )
    ).toBe(true);
    expect(
      invalidYamlValidation.data?.issues.some(
        (issue) =>
          issue.code === 'media_yaml_policy_profile_not_found' &&
          issue.pointer === '/profiles/0/policy_key' &&
          issue.blocking
      )
    ).toBe(true);

    const invalidYamlApply = await api.POST('/v1/media/imports/apply', {
      body: { yaml_payload: invalidYamlPayload, preconditions: [] },
    });
    expect(invalidYamlApply.response.status).toBe(400);

    const portableApply = await api.POST('/v1/media/imports/apply', {
      body: { yaml_payload: yamlPayload, preconditions: fixtureImportPreconditions(yamlPayload) },
    });
    expect(portableApply.response.status, JSON.stringify(portableApply.error)).toBe(201);
    expect(portableApply.data?.forced_dry_run).toBe(true);

    const localExported = await api.GET('/v1/media/export', {
      params: { query: { include_local_paths: true } },
    });
    expect(localExported.response.status).toBe(200);
    const localYamlPayload = localExported.data?.yaml_payload;
    if (!localYamlPayload) {
      throw new Error('Missing local media YAML payload');
    }

    const localSnapshot = loadYaml(localYamlPayload) as {
      kind: string; local_root_paths: { logical_key: string; canonical_path: string | null }[];
    };
    expect(localSnapshot.kind).toBe('revaer.media.local_snapshot');
    expect(localSnapshot.local_root_paths.some(row => row.logical_key === 'source'
      && row.canonical_path === sourceRoot)).toBe(true);
    expect(localSnapshot.local_root_paths.some(row => row.logical_key === 'workspace'
      && row.canonical_path !== null)).toBe(true);
    const applied = await api.POST('/v1/media/imports/apply', {
      body: { yaml_payload: localYamlPayload, preconditions: fixtureImportPreconditions(yamlPayload) },
    });
    expect(applied.response.status).toBe(400);
    expect(fixture.readSource()).toEqual(fixture.sourceBytes);
    const afterLocalImport = await api.GET('/v1/media/export');
    expect(afterLocalImport.response.status).toBe(200);
    expect(afterLocalImport.data?.yaml_payload).toBe(yamlPayload);

    const currentProfile = await api.GET('/v1/media/profiles/{media_profile_public_id}', {
      params: { path: { media_profile_public_id: profileId } },
    });
    expect(currentProfile.response.status, JSON.stringify(currentProfile.error)).toBe(200);
    const currentTag = currentProfile.response.headers.get('etag');
    if (!currentTag) throw new Error('Missing post-import profile fence');
    const restoredProfile = await api.PUT('/v1/media/profiles/{media_profile_public_id}', {
      params: { path: { media_profile_public_id: profileId }, header: { 'If-Match': currentTag } },
      body: updatedBody,
    });
    expect(restoredProfile.response.status, JSON.stringify(restoredProfile.error)).toBe(200);
    expect(restoredProfile.data).toMatchObject(updatedBody);
    expect(restoredProfile.data?.latest_version).toBe(4);
    const association = await api.POST('/v1/media/discovery-associations', {
      params: { header: { 'If-None-Match': '*' } },
      body: { association_key: suffix, media_profile_public_id: profileId, profile_version: 4,
        source_root_key: 'source', root_relative_path: suffix, manual_enabled: true,
        watcher_enabled: false, schedule_enabled: false },
    });
    expect(association.response.status, JSON.stringify(association.error)).toBe(201);
    const associationId = association.data?.media_discovery_association_public_id;
    if (!associationId) throw new Error('Missing native discovery association identity');
    const schedules = await api.GET('/v1/media/discovery/schedules');
    expect(schedules.response.status, JSON.stringify(schedules.error)).toBe(200);
    expect(schedules.data?.schedules.find(row =>
      row.media_discovery_association_public_id === associationId)).toEqual(association.data);
    const watchers = await api.GET('/v1/media/discovery/watchers');
    expect(watchers.response.status, JSON.stringify(watchers.error)).toBe(200);
    expect(watchers.data?.watchers.find(row =>
      row.media_discovery_association_public_id === associationId)).toEqual(association.data);
    for (const endpoint of ['/v1/media/discovery/watchers', '/v1/media/discovery/schedules'] as const) {
      const disabled = await api.POST(endpoint, {
        body: { media_discovery_association_public_id: associationId, source_paths: [sourcePath] },
      });
      expect(disabled.response.status, JSON.stringify(disabled.error)).toBe(400);
    }

    const planningPreview = await api.POST('/v1/media/planning/preview', {
      body: {
        media_discovery_association_public_id: associationId,
        source_path: sourcePath,
      },
    });
    expect(planningPreview.response.status).toBe(200);
    expect(planningPreview.data?.accepted).toBe(true);

    const preview = await api.POST('/v1/media/discovery/preview', {
      body: {
        media_discovery_association_public_id: associationId,
        source_paths: [sourcePath],
      },
    });
    expect(preview.response.status).toBe(200);
    expect(preview.data?.previews[0]?.accepted).toBe(true);

    const discoveryRun = await api.POST('/v1/media/discovery/runs', {
      body: {
        media_discovery_association_public_id: associationId,
        source_paths: [manualPath],
      },
    });
    expect(discoveryRun.response.status, JSON.stringify(discoveryRun.error)).toBe(201);
    expect(
      (discoveryRun.data?.queued_jobs.length ?? 0) + (discoveryRun.data?.skipped.length ?? 0)
    ).toBe(1);
    if (discoveryRun.data?.queued_jobs.length === 0) {
      expect([
        'media_discovery_source_unchanged',
        'media_discovery_source_unstable',
      ]).toContain(discoveryRun.data?.skipped[0]?.reason);
    }
    const manualRunQueuedJob = discoveryRun.data?.queued_jobs.length === 1;
    const duplicateDiscoveryRun = await api.POST('/v1/media/discovery/runs', {
      body: {
        media_discovery_association_public_id: associationId,
        source_paths: [manualPath],
      },
    });
    expect(duplicateDiscoveryRun.response.status).toBe(201);
    if (manualRunQueuedJob) {
      expect(duplicateDiscoveryRun.data?.queued_jobs.length).toBe(0);
      expect(duplicateDiscoveryRun.data?.skipped[0]?.reason).toBe(
        'media_discovery_source_unchanged'
      );
    } else {
      expect(
        (duplicateDiscoveryRun.data?.queued_jobs.length ?? 0) +
          (duplicateDiscoveryRun.data?.skipped.length ?? 0)
      ).toBe(1);
      if (duplicateDiscoveryRun.data?.queued_jobs.length === 0) {
        expect([
          'media_discovery_source_unchanged',
          'media_discovery_source_unstable',
        ]).toContain(duplicateDiscoveryRun.data?.skipped[0]?.reason);
      }
    }

    const jobId = discoveryRun.data?.queued_jobs[0]?.media_job_public_id;
    if (!jobId) {
      throw new Error('Missing media job public id');
    }

    const jobs = await api.GET('/v1/media/jobs', {
      params: { query: { media_profile_public_id: profileId } },
    });
    expect(jobs.response.status).toBe(200);
    expect(jobs.data?.jobs.map((job) => job.media_job_public_id) ?? []).toContain(jobId);
    const listedJob = jobs.data?.jobs.find(row => row.media_job_public_id === jobId);
    expect(listedJob?.source_path).toBe(manualPath);
    expect(listedJob?.output_path).toBe(manualPath);

    const job = await api.GET('/v1/media/jobs/{media_job_public_id}', {
      params: { path: { media_job_public_id: jobId } },
    });
    expect(job.response.status).toBe(200);
    expect(job.data?.source_path).toBe(manualPath);
    expect(job.data?.output_path).toBe(manualPath);
    const recent = await api.GET('/v1/media/jobs/recent', {
      params: { query: { limit: 1, media_profile_public_id: profileId } },
    });
    expect(recent.response.status, JSON.stringify(recent.error)).toBe(200);
    expect(recent.data?.jobs[0]?.media_job_public_id).toBe(jobId);
    expect(recent.data?.jobs[0]?.source_path).toBe(manualPath);
    expect(recent.data?.jobs[0]?.output_path).toBe(manualPath);
    expect(JSON.stringify({ listedJob, job: job.data, recent: recent.data })).not.toContain('/proof/');

    const phases = await api.GET('/v1/media/jobs/{media_job_public_id}/phases', {
      params: { path: { media_job_public_id: jobId } },
    });
    expect(phases.response.status).toBe(200);
    expect(Array.isArray(phases.data?.phases)).toBe(true);
    const operations = await api.GET('/v1/media/jobs/{media_job_public_id}/operations', {
      params: { path: { media_job_public_id: jobId } },
    });
    expect(operations.response.status).toBe(200);
    expect(Array.isArray(operations.data?.operations)).toBe(true);

    const violations = await api.GET('/v1/media/jobs/{media_job_public_id}/violations', {
      params: { path: { media_job_public_id: jobId } },
    });
    expect(violations.response.status).toBe(200);
    expect(Array.isArray(violations.data?.violations)).toBe(true);

    const reasons = await api.GET('/v1/media/jobs/{media_job_public_id}/plan-reasons', {
      params: { path: { media_job_public_id: jobId } },
    });
    expect(reasons.response.status).toBe(200);
    expect(Array.isArray(reasons.data?.reasons)).toBe(true);

    const checks = await api.GET('/v1/media/jobs/{media_job_public_id}/verification-checks', {
      params: { path: { media_job_public_id: jobId } },
    });
    expect(checks.response.status).toBe(200);
    expect(Array.isArray(checks.data?.checks)).toBe(true);

    const artifacts = await api.GET('/v1/media/jobs/{media_job_public_id}/artifacts', {
      params: { path: { media_job_public_id: jobId } },
    });
    expect(artifacts.response.status).toBe(200);
    expect(Array.isArray(artifacts.data?.artifacts)).toBe(true);

    const audits = await api.GET('/v1/media/jobs/{media_job_public_id}/compact-audits', {
      params: { path: { media_job_public_id: jobId } },
    });
    expect(audits.response.status).toBe(200);
    expect(Array.isArray(audits.data?.audits)).toBe(true);

    const cancel = await api.POST('/v1/media/jobs/{media_job_public_id}/cancel', {
      params: { path: { media_job_public_id: jobId } },
    });
    expect([204, 409]).toContain(cancel.response.status);

    const retry = await api.POST('/v1/media/jobs/{media_job_public_id}/retry', {
      params: { path: { media_job_public_id: jobId } },
    });
    expect([204, 409]).toContain(retry.response.status);
  });

  for (const mode of ['watcher', 'schedule'] as const) {
    test(`activates native ${mode} discovery and admits a dry-run job`, async ({ api, profileFixture: fixture }) => {
      const created = await api.POST('/v1/media/profiles', {
        params: { header: { 'If-None-Match': '*' } }, body: fixture.request,
      });
      expect(created.response.status, JSON.stringify(created.error)).toBe(201);
      const profileId = created.data?.media_profile_public_id;
      if (!profileId) throw new Error('Missing automation profile identity');
      const association = await api.POST('/v1/media/discovery-associations', {
        params: { header: { 'If-None-Match': '*' } },
        body: { association_key: fixture.prefix, media_profile_public_id: profileId, profile_version: 1,
          source_root_key: 'source', root_relative_path: fixture.prefix, manual_enabled: false,
          watcher_enabled: mode === 'watcher', schedule_enabled: mode === 'schedule' },
      });
      expect(association.response.status, JSON.stringify(association.error)).toBe(201);
      const associationId = association.data?.media_discovery_association_public_id;
      if (!associationId) throw new Error('Missing automation association identity');
      const route = mode === 'watcher' ? '/v1/media/discovery/watchers' : '/v1/media/discovery/schedules';
      const run = await api.POST(route, {
        body: { media_discovery_association_public_id: associationId, source_paths: [fixture.relativePath] },
      });
      expect(run.response.status, JSON.stringify(run.error)).toBe(201);
      expect(run.data?.skipped).toEqual([]);
      expect(run.data?.queued_jobs).toHaveLength(1);
      expect(run.data?.queued_jobs[0]?.dry_run).toBe(true);
      expect(fixture.readSource()).toEqual(fixture.sourceBytes);
    });
  }
});
