import { test, expect } from '../../fixtures/api';

test.describe('Root catalog readiness', () => {
  test('lists persisted missing catalog without inventing slots', async ({ api }) => {
    const result = await api.GET('/v1/media/root-catalog');
    expect(result.response.status, JSON.stringify(result.error)).toBe(200);
    expect(result.response.headers.get('cache-control')).toBe('no-store');
    expect(result.data).toEqual({
      format_version: 1,
      source_state: 'missing',
      source_reason: 'media_root_catalog_source_missing',
      attestation_state: 'not_evaluated',
      slots: [],
    });
  });

  test('rejects an out-of-bounds catalog page without returning paths', async ({ api }) => {
    const result = await api.GET('/v1/media/root-catalog', { params: { query: { limit: 0 } } });
    expect(result.response.status, JSON.stringify(result.error)).toBe(400);
    expect(result.response.headers.get('cache-control')).toBe('no-store');
    expect(result.error).toMatchObject({
      context: [{ name: 'error_code', value: 'media_configuration_invalid' }],
    });
    expect(result.response.headers.get('content-type')).toBe('application/problem+json');
  });

  test('reports persisted unconfigured state without inventing attestation', async ({ api }) => {
    const result = await api.GET('/v1/media/root-catalog/readiness');
    expect(result.response.status, JSON.stringify(result.error)).toBe(200);
    expect(result.response.headers.get('cache-control')).toBe('no-store');
    expect(result.data).toEqual({
      format_version: 1,
      source_state: 'missing',
      source_reason: 'media_root_catalog_source_missing',
      attestation_state: 'not_evaluated',
      kinds: ['source', 'output', 'workspace', 'backup', 'quarantine'].map((kind) => ({
        kind,
        attested_slot_count: 0,
        binding_ready_slot_count: 0,
        destructive_ready_slot_count: 0,
      })),
    });
  });
});
