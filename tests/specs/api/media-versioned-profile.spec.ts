import { randomUUID } from 'node:crypto';
import { test, expect } from '../../fixtures/media';

test('persists a catalog-backed dry-run profile with an exact version fence', async ({ api }) => {
  const key = `versioned-${randomUUID()}`;
  const target = await api.POST('/v1/media/targets', { body: {
    target_key: key, version: 1, display_name: 'Versioned fixture target', container_format: 'matroska',
    streams: [{ stream_key: 'main-video', stream_kind: 'video', optional: false, sort_order: 0,
      codec: 'h264', default_disposition: true, forced_disposition: false }],
  } });
  expect(target.response.status, JSON.stringify(target.error)).toBe(201);
  const policy = await api.POST('/v1/media/policies', { body: {
    policy_key: key, version: 1, display_name: 'Versioned fixture policy', video_intent: 'general',
    verification_strictness: 'strict', verification_duration_tolerance_millis: 100,
    verification_mux_validation: true, verification_decode_all_streams: true,
    verification_keyframe_seek: true, verification_playback_probe: true,
    output: { dry_run: true, replacement_mode: 'atomic_replace', quarantine_enabled: true,
      preserve_permissions: true, preserve_ownership: true },
  } });
  expect(policy.response.status, JSON.stringify(policy.error)).toBe(201);
  const body = { profile_key: key, display_name: 'Versioned fixture', description: '', enabled: true, dry_run_only: true,
    desired_target_key: key, desired_target_version: 1, policy_key: key, policy_version: 1,
    output_root_key: 'source', workspace_root_key: 'workspace', quarantine_root_key: 'quarantine' };
  const created = await api.POST('/v1/media/profiles', {
    params: { header: { 'If-None-Match': '*' } }, body,
  });
  expect(created.response.status, JSON.stringify(created.error)).toBe(201);
  expect(created.data).toMatchObject(body);
  const id = created.data?.media_profile_public_id;
  if (!id) throw new Error('Missing immutable profile identity');
  const tag = `"media-profile:${id}:v1"`;
  expect(created.response.headers.get('etag')).toBe(tag);
  const read = await api.GET('/v1/media/profiles/{media_profile_public_id}', {
    params: { path: { media_profile_public_id: id } },
  });
  expect(read.response.status, JSON.stringify(read.error)).toBe(200);
  expect(read.data).toEqual(created.data);
  expect(read.response.headers.get('etag')).toBe(tag);
  const replacement = { ...body, description: 'Complete conditional replacement' };
  const replaced = await api.PUT('/v1/media/profiles/{media_profile_public_id}', {
    params: { path: { media_profile_public_id: id }, header: { 'If-Match': tag } }, body: replacement,
  });
  expect(replaced.response.status, JSON.stringify(replaced.error)).toBe(200);
  expect(replaced.data).toMatchObject(replacement);
  expect(replaced.response.headers.get('etag')).toBe(`"media-profile:${id}:v2"`);
  const stale = await api.PUT('/v1/media/profiles/{media_profile_public_id}', {
    params: { path: { media_profile_public_id: id }, header: { 'If-Match': tag } },
    body: { ...replacement, description: 'Must not persist' },
  });
  expect(stale.response.status, JSON.stringify(stale.error)).toBe(412);
  const unchanged = await api.GET('/v1/media/profiles/{media_profile_public_id}', {
    params: { path: { media_profile_public_id: id } },
  });
  expect(unchanged.response.status, JSON.stringify(unchanged.error)).toBe(200);
  expect(unchanged.data).toEqual(replaced.data);
});
