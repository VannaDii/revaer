import { test, expect } from '../../fixtures/media';
import { authHeaders } from '../../support/headers';

for (const unit of ['minutes', 'hours'] as const) {
  test(`saves explicit ${unit} cadence without enabling automation or changing media`, async ({ api, baseUrl, session, profileFixture: fixture }) => {
    const profile = await api.POST('/v1/media/profiles', {
      params: { header: { 'If-None-Match': '*' } }, body: fixture.request,
    });
    expect(profile.response.status, JSON.stringify(profile.error)).toBe(201);
    const profileId = profile.data?.media_profile_public_id;
    if (!profileId) throw new Error('Missing saved profile identity');
    const association = await api.POST('/v1/media/discovery-associations', {
      params: { header: { 'If-None-Match': '*' } },
      body: { association_key: fixture.prefix, media_profile_public_id: profileId, profile_version: 1,
        source_root_key: 'source', root_relative_path: fixture.prefix, manual_enabled: true,
        watcher_enabled: false, schedule_enabled: false },
    });
    expect(association.response.status, JSON.stringify(association.error)).toBe(201);
    const id = association.data?.media_discovery_association_public_id;
    if (!id) throw new Error('Missing saved association identity');
    const url = `${baseUrl}/v1/media/discovery-associations/${id}/schedule`;
    const headers = { ...authHeaders(session), 'Content-Type': 'application/json' };
    const selected = { association_version: 1, interval_quantity: 2, interval_unit: unit };
    const params = { path: { media_discovery_association_public_id: id } };
    const absent = await api.GET('/v1/media/discovery-associations/{media_discovery_association_public_id}/schedule', { params });
    expect(absent.response.status).toBe(404);
    const missingFence = await fetch(url, { method: 'POST', headers, body: JSON.stringify(selected) });
    expect(missingFence.status).toBe(428);
    const conditionalHeaders = { ...headers, 'If-None-Match': '*' };
    for (const invalid of [
      {}, { ...selected, interval_quantity: 0 },
      { ...selected, interval_quantity: unit === 'minutes' ? 43201 : 721 },
      { ...selected, interval_unit: 'days' }, { ...selected, source_root: '/media' },
    ]) {
      const rejected = await fetch(url, { method: 'POST', headers: conditionalHeaders, body: JSON.stringify(invalid) });
      expect(rejected.status, await rejected.text()).toBe(400);
      expect((await fetch(url, { headers })).status).toBe(404);
    }
    const stale = await fetch(url, { method: 'POST', headers: conditionalHeaders,
      body: JSON.stringify({ ...selected, association_version: 2 }) });
    expect(stale.status, await stale.text()).toBe(409);
    const saved = await api.POST('/v1/media/discovery-associations/{media_discovery_association_public_id}/schedule', {
      params: { ...params, header: { 'If-None-Match': '*' } }, body: selected,
    });
    expect(saved.response.status, JSON.stringify(saved.error)).toBe(201);
    expect(saved.response.headers.get('cache-control')).toBe('no-store');
    const body = saved.data;
    if (!body) throw new Error('Missing saved schedule configuration');
    expect(body).toMatchObject({ ...selected, media_discovery_association_public_id: id });
    expect(Number.isFinite(Date.parse(body.anchor_due_at))).toBe(true);
    const duplicate = await fetch(url, { method: 'POST', headers: conditionalHeaders,
      body: JSON.stringify({ ...selected, interval_quantity: 3 }) });
    expect(duplicate.status, await duplicate.text()).toBe(409);
    const persisted = await api.GET('/v1/media/discovery-associations/{media_discovery_association_public_id}/schedule', { params });
    expect(persisted.response.status, JSON.stringify(persisted.error)).toBe(200);
    expect(persisted.data).toEqual(body);
    const fence = persisted.response.headers.get('etag');
    if (!fence) throw new Error('Missing schedule revision fence');
    expect(saved.response.headers.get('etag')).toBe(fence);
    const changed = { ...selected, interval_quantity: 3 };
    for (const invalidFence of [undefined, '*', `W/${fence}`, `${fence}, ${fence}`]) {
      const rejected = await fetch(url, { method: 'PUT',
        headers: { ...headers, ...(invalidFence ? { 'If-Match': invalidFence } : {}) }, body: 'invalid json' });
      expect(rejected.status, await rejected.text()).toBe(invalidFence ? 400 : 428);
    }
    const edited = await api.PUT('/v1/media/discovery-associations/{media_discovery_association_public_id}/schedule', {
      params: { ...params, header: { 'If-Match': fence } }, body: changed,
    });
    expect(edited.response.status, JSON.stringify(edited.error)).toBe(200);
    expect(edited.data).toMatchObject({ ...changed, anchor_due_at: body.anchor_due_at });
    expect(edited.data?.updated_at).not.toBe(body.updated_at);
    expect(edited.response.headers.get('etag')).not.toBe(fence);
    const staleEdit = await api.PUT('/v1/media/discovery-associations/{media_discovery_association_public_id}/schedule', {
      params: { ...params, header: { 'If-Match': fence } }, body: { ...changed, interval_quantity: 4 },
    });
    expect(staleEdit.response.status, JSON.stringify(staleEdit.error)).toBe(412);
    const afterEdit = await api.GET('/v1/media/discovery-associations/{media_discovery_association_public_id}/schedule', { params });
    expect(afterEdit.data).toEqual(edited.data);
    expect(afterEdit.response.headers.get('etag')).toBe(edited.response.headers.get('etag'));
    const unchanged = await api.GET('/v1/media/discovery-associations/{media_discovery_association_public_id}', {
      params: { path: { media_discovery_association_public_id: id } },
    });
    expect(unchanged.response.status, JSON.stringify(unchanged.error)).toBe(200);
    expect(unchanged.data).toEqual(association.data);
    expect(fixture.readSource()).toEqual(fixture.sourceBytes);
  });
}
