import { test, expect } from '../../fixtures/api';

function policy() {
  return {
    policy_key: `output-${crypto.randomUUID()}`, version: 1, display_name: 'Output contract',
    video_intent: 'general', verification_strictness: 'strict',
    verification_duration_tolerance_millis: 100,
    verification_mux_validation: true, verification_decode_all_streams: true,
    verification_keyframe_seek: true, verification_playback_probe: true,
    output: { dry_run: false, replacement_mode: 'atomic_replace' as const, quarantine_enabled: false,
      preserve_permissions: false, preserve_ownership: true },
  };
}

test.describe('Complete immutable output policy', () => {
  test('creates and reads exact execution settings without replacing a version', async ({ api }) => {
    const request = policy();
    const saved = await api.POST('/v1/media/policies', { body: request });
    expect(saved.response.status, JSON.stringify(saved.error)).toBe(201);
    expect(saved.data?.output).toEqual(request.output);
    const read = await api.GET('/v1/media/policies');
    expect(read.response.status, JSON.stringify(read.error)).toBe(200);
    expect(read.data?.policies.find(row => row.policy_key === request.policy_key)?.output).toEqual(request.output);
    const duplicate = await api.POST('/v1/media/policies', {
      body: { ...request, output: { ...request.output, dry_run: true } },
    });
    expect(duplicate.response.status).toBe(409);
    const unchanged = await api.GET('/v1/media/policies');
    expect(unchanged.data?.policies.find(row => row.policy_key === request.policy_key)?.output).toEqual(request.output);
  });

  test('rejects unsafe and unauthorized modes without leaving a policy row', async ({ api, request: http, session, baseUrl }) => {
    for (const replacement_mode of ['disabled', 'side_by_side', 'unrecognized']) {
      const request = policy();
      const result = await http.post(`${baseUrl}/v1/media/policies`, {
        headers: session.apiKey ? { 'x-revaer-api-key': session.apiKey } : {},
        data: { ...request, output: { ...request.output, replacement_mode } },
      });
      expect(result.status(), await result.text()).toBe(400);
      const read = await api.GET('/v1/media/policies');
      expect(read.response.status).toBe(200);
      expect(read.data?.policies.some(row => row.policy_key === request.policy_key)).toBe(false);
    }
  });
});
