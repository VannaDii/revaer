import { randomUUID } from 'node:crypto';
import { test, expect } from '../../fixtures/api';

test.describe('Media API', () => {
  test('covers media profiles jobs capabilities discovery and diagnostics', async ({ api }) => {
    const suffix = randomUUID().slice(0, 8);
    const sourceRoot = `/tmp/revaer-media-e2e-${suffix}/source`;
    const outputRoot = `/tmp/revaer-media-e2e-${suffix}/output`;
    const sourcePath = `${sourceRoot}/movie.mkv`;
    const outputPath = `${outputRoot}/movie.mkv`;

    const createdProfile = await api.POST('/v1/media/profiles', {
      body: {
        profile_key: `e2e-media-${suffix}`,
        source_root: sourceRoot,
        output_root: outputRoot,
        dry_run_only: true,
        retention_days: 30,
        schedule_enabled: true,
        schedule_interval_minutes: 60,
        watcher_enabled: false,
      },
    });
    expect(createdProfile.response.status).toBe(201);
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
    expect(profile.data?.source_root).toBe(sourceRoot);

    const patchedProfile = await api.PATCH('/v1/media/profiles/{media_profile_public_id}', {
      params: { path: { media_profile_public_id: profileId } },
      body: {
        retention_days: 31,
        schedule_interval_minutes: 120,
      },
    });
    expect(patchedProfile.response.status).toBe(200);
    expect(patchedProfile.data?.retention_days).toBe(31);

    const validatedProfile = await api.POST('/v1/media/profiles/validate', {
      body: {
        profile_key: `e2e-media-validated-${suffix}`,
        source_root: sourceRoot,
        output_root: outputRoot,
        dry_run_only: true,
        retention_days: 30,
        schedule_enabled: true,
        schedule_interval_minutes: 60,
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

    const pinnedDesiredTarget = await api.PATCH(
      '/v1/media/profiles/{media_profile_public_id}/desired-target',
      {
        params: { path: { media_profile_public_id: profileId } },
        body: { target_key: desiredTargetKey, version: 1 },
      }
    );
    expect(pinnedDesiredTarget.response.status).toBe(204);

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
      `    source_root: ${sourceRoot}/yaml-invalid`,
      `    output_root: ${outputRoot}/yaml-invalid`,
      '    dry_run_only: true',
      '    retention_days: 30',
      `    compatibility_target_key: missing-target-${suffix}`,
      `    policy_key: missing-policy-${suffix}`,
    ].join('\n');
    const invalidYamlValidation = await api.POST('/v1/media/imports/validate', {
      body: { yaml_payload: invalidYamlPayload },
    });
    expect(invalidYamlValidation.response.status).toBe(200);
    expect(invalidYamlValidation.data?.valid).toBe(false);
    expect(
      invalidYamlValidation.data?.issues.some(
        (issue) =>
          issue.code === 'media_yaml_compatibility_target_not_found' &&
          issue.pointer === '/profiles/0/compatibility_target_key' &&
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
      body: { yaml_payload: invalidYamlPayload },
    });
    expect(invalidYamlApply.response.status).toBe(400);

    const portableApply = await api.POST('/v1/media/imports/apply', {
      body: { yaml_payload: yamlPayload },
    });
    expect(portableApply.response.status).toBe(201);
    expect(portableApply.data?.forced_dry_run).toBe(true);

    const localExported = await api.GET('/v1/media/export', {
      params: { query: { include_local_paths: true } },
    });
    expect(localExported.response.status).toBe(200);
    const localYamlPayload = localExported.data?.yaml_payload;
    if (!localYamlPayload) {
      throw new Error('Missing local media YAML payload');
    }

    const applied = await api.POST('/v1/media/imports/apply', {
      body: { yaml_payload: localYamlPayload },
    });
    expect(applied.response.status).toBe(201);
    expect(applied.data?.forced_dry_run).toBe(true);

    const restoredProfile = await api.PATCH('/v1/media/profiles/{media_profile_public_id}', {
      params: { path: { media_profile_public_id: profileId } },
      body: {
        source_root: sourceRoot,
        output_root: outputRoot,
        dry_run_only: true,
        retention_days: 31,
        schedule_enabled: false,
        schedule_interval_minutes: 120,
        watcher_enabled: false,
      },
    });
    expect(restoredProfile.response.status).toBe(200);
    expect(restoredProfile.data?.source_root).toBe(sourceRoot);
    expect(restoredProfile.data?.output_root).toBe(outputRoot);

    const schedules = await api.GET('/v1/media/discovery/schedules');
    expect(schedules.response.status).toBe(200);
    const scheduleEntry = schedules.data?.schedules.find(
      (schedule) => schedule.media_profile_public_id === profileId
    );
    expect(scheduleEntry).toBeTruthy();
    expect(scheduleEntry?.enabled).toBe(false);

    const scheduledProfile = await api.PATCH('/v1/media/profiles/{media_profile_public_id}', {
      params: { path: { media_profile_public_id: profileId } },
      body: {
        schedule_enabled: true,
        schedule_interval_minutes: 120,
      },
    });
    expect(scheduledProfile.response.status).toBe(200);
    expect(scheduledProfile.data?.schedule_enabled).toBe(true);

    const watchers = await api.GET('/v1/media/discovery/watchers');
    expect(watchers.response.status).toBe(200);
    expect(
      watchers.data?.watchers.some(
        (watcher) => watcher.media_profile_public_id === profileId && watcher.enabled === false
      )
    ).toBeTruthy();

    const watcherProfile = await api.PATCH('/v1/media/profiles/{media_profile_public_id}', {
      params: { path: { media_profile_public_id: profileId } },
      body: {
        watcher_enabled: true,
      },
    });
    expect(watcherProfile.response.status).toBe(200);
    expect(watcherProfile.data?.watcher_enabled).toBe(true);

    const enabledWatchers = await api.GET('/v1/media/discovery/watchers');
    expect(enabledWatchers.response.status).toBe(200);
    expect(
      enabledWatchers.data?.watchers.some(
        (watcher) => watcher.media_profile_public_id === profileId && watcher.enabled === true
      )
    ).toBeTruthy();

    const watcherRun = await api.POST('/v1/media/discovery/watchers', {
      body: {
        media_profile_public_id: profileId,
        source_paths: [`${sourceRoot}/watcher-${suffix}.mkv`],
      },
    });
    expect(watcherRun.response.status).toBe(201);
    expect(watcherRun.data?.queued_jobs.length).toBe(1);

    const planningPreview = await api.POST('/v1/media/planning/preview', {
      body: {
        media_profile_public_id: profileId,
        source_path: sourcePath,
      },
    });
    expect(planningPreview.response.status).toBe(200);
    expect(planningPreview.data?.accepted).toBe(true);

    const preview = await api.POST('/v1/media/discovery/preview', {
      body: {
        media_profile_public_id: profileId,
        source_paths: [sourcePath],
      },
    });
    expect(preview.response.status).toBe(200);
    expect(preview.data?.previews[0]?.accepted).toBe(true);

    const discoveryRun = await api.POST('/v1/media/discovery/runs', {
      body: {
        media_profile_public_id: profileId,
        source_paths: [`${sourceRoot}/manual-${suffix}.mkv`],
      },
    });
    expect(discoveryRun.response.status).toBe(201);
    expect(discoveryRun.data?.queued_jobs.length).toBe(1);

    const scheduleRun = await api.POST('/v1/media/discovery/schedules', {
      body: {
        media_profile_public_id: profileId,
        source_paths: [`${sourceRoot}/schedule-${suffix}.mkv`],
      },
    });
    expect(scheduleRun.response.status).toBe(201);

    const createdJob = await api.POST('/v1/media/jobs', {
      body: {
        media_profile_public_id: profileId,
        source_path: sourcePath,
        output_path: outputPath,
        dry_run: true,
      },
    });
    expect(createdJob.response.status).toBe(201);
    const jobId = createdJob.data?.media_job_public_id;
    if (!jobId) {
      throw new Error('Missing media job public id');
    }

    const jobs = await api.GET('/v1/media/jobs', {
      params: { query: { media_profile_public_id: profileId, status: 'queued' } },
    });
    expect(jobs.response.status).toBe(200);
    expect(jobs.data?.jobs.some((job) => job.media_job_public_id === jobId)).toBeTruthy();

    const job = await api.GET('/v1/media/jobs/{media_job_public_id}', {
      params: { path: { media_job_public_id: jobId } },
    });
    expect(job.response.status).toBe(200);
    expect(job.data?.source_path).toBe(sourcePath);

    const phase = await api.POST('/v1/media/jobs/{media_job_public_id}/phases', {
      params: { path: { media_job_public_id: jobId } },
      body: {
        phase_index: 90,
        phase_name: 'inspect',
        phase_status: 'running',
        details_text: 'inspection started',
      },
    });
    expect(phase.response.status).toBe(204);

    const phases = await api.GET('/v1/media/jobs/{media_job_public_id}/phases', {
      params: { path: { media_job_public_id: jobId } },
    });
    expect(phases.response.status).toBe(200);
    expect(phases.data?.phases.some((item) => item.phase_name === 'inspect')).toBeTruthy();

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
});
