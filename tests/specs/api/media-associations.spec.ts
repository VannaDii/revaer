import { test, expect } from '../../fixtures/api';

test.describe('Immutable discovery association collection', () => {
  test('reads a bounded path-free page from the real service', async ({ api }) => {
    const result = await api.GET('/v1/media/discovery-associations', { params: { query: { limit: 1 } } });
    expect(result.response.status, JSON.stringify(result.error)).toBe(200);
    expect(result.response.headers.get('cache-control')).toBe('no-store');
    expect(result.data?.associations).toEqual([]);
    expect(result.data?.next_cursor).toBeUndefined();
  });

  test('rejects invalid bounds and foreign continuation collections', async ({ api }) => {
    for (const query of [{ limit: 0 }, { limit: 201 }, { cursor: 'profiles_bad' }, { cursor: 'associations_bad' }]) {
      const result = await api.GET('/v1/media/discovery-associations', { params: { query } });
      expect(result.response.status, JSON.stringify(result.error)).toBe(400);
      expect(result.response.headers.get('cache-control')).toBe('no-store');
      expect(result.response.headers.get('content-type')).toBe('application/problem+json');
      expect(result.error).toMatchObject({ context: [{ name: 'error_code', value: 'media_configuration_invalid' }] });
    }
  });
});
