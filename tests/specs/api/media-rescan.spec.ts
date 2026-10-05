import { test, expect } from '../../fixtures/media';
import { execFileSync } from 'node:child_process';

test('activation retains a bounded coalesced rescan across real service restart', async ({ api, mediaService, profileFixture: fixture }) => {
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
  const command = ['exec', '-i', mediaService.container.slice(0, -4), 'psql', '-X', '-qAt',
    '-U', 'postgres', '-d', mediaService.database, '-v', 'ON_ERROR_STOP=1'];
  const sql = (input: string) => execFileSync('docker', command, { input, encoding: 'utf8', timeout: 10000 }).trim();
  const read = () => sql(`SELECT runtime_role FROM revaer_system.database_baseline \u005cgset
    SET ROLE :runtime_role;
    SELECT association_version, requested_sequence, satisfied_sequence, reason_code, last_requested_sequence
    FROM public.media_discovery_rescan_get_v1('${id}');`);
  expect(read()).toBe('1|1|0|configuration_activated|1');
  const version = `SELECT latest_media_discovery_association_version_id FROM public.media_discovery_association
    WHERE media_discovery_association_public_id = '${id}'`;
  expect(sql(`SELECT public.media_discovery_rescan_publish_v1((${version}), 'overflow');
    SELECT public.media_discovery_rescan_publish_v1((${version}), 'overflow');`)).toBe('2\n3');
  expect(read()).toBe('1|3|0|configuration_activated|1\n1|3|0|overflow|3');
  expect(sql(`DO $$ BEGIN
    BEGIN
      PERFORM public.media_discovery_rescan_publish_v1((${version}), 'unknown');
      RAISE EXCEPTION 'unknown reason was accepted';
    EXCEPTION WHEN SQLSTATE 'P0001' THEN
      IF SQLERRM <> 'media_configuration_invalid' THEN RAISE; END IF;
    END;
  END $$;
  SELECT requested_sequence FROM public.media_discovery_rescan WHERE media_discovery_association_version_id = (${version});`)).toBe('3');
  expect(sql(`SELECT count(*) FROM public.media_discovery_rescan WHERE media_discovery_association_version_id = (${version});
    SELECT count(*) FROM public.media_discovery_rescan_reason WHERE media_discovery_association_version_id = (${version});
    SELECT runtime_role FROM revaer_system.database_baseline \u005cgset
    SELECT has_function_privilege(:'runtime_role', 'public.media_discovery_rescan_publish_v1(bigint,text)', 'EXECUTE');
    SELECT has_table_privilege(:'runtime_role', 'public.media_discovery_rescan', 'UPDATE');`)).toBe('1\n2\nf\nf');
  expect(sql(`SELECT public.media_discovery_rescan_publish_v1((${version}), reason_code)
    FROM public.media_discovery_rescan_reason_kind WHERE reason_code NOT IN ('configuration_activated', 'overflow')
    ORDER BY reason_code COLLATE "C";
    SELECT count(*) FROM public.media_discovery_rescan_reason WHERE media_discovery_association_version_id = (${version});`)).toBe('4\n5\n6\n7\n8\n7');
  expect(sql(`BEGIN;
    UPDATE public.media_discovery_rescan SET requested_sequence = 9223372036854775807
    WHERE media_discovery_association_version_id = (${version});
    DO $$ BEGIN
      BEGIN
        PERFORM public.media_discovery_rescan_publish_v1((${version}), 'overflow');
        RAISE EXCEPTION 'sequence overflow was accepted';
      EXCEPTION WHEN numeric_value_out_of_range THEN NULL;
      END;
    END $$;
    SELECT requested_sequence FROM public.media_discovery_rescan WHERE media_discovery_association_version_id = (${version});
    ROLLBACK;`)).toBe('9223372036854775807');
  const beforeRestart = read();
  execFileSync('docker', ['exec', mediaService.container, 'touch', '/proof/restart']);
  await expect.poll(() => execFileSync('docker', ['exec', mediaService.container, 'bash', '-c',
    'if [[ -f /proof/restarted-pid ]]; then cat /proof/restarted-pid; fi'], { encoding: 'utf8', timeout: 5000 }).trim(),
  { timeout: 20000 }).toMatch(/^[1-9][0-9]*$/);
  await expect.poll(async () => (await api.GET('/health')).response.status, { timeout: 20000 }).toBe(200);
  expect(read()).toBe(beforeRestart);
  expect(fixture.readSource()).toEqual(fixture.sourceBytes);
  // ADR 594 retains rescan requests under the service's existing root lock.
  // No scanner owner, lease expiry or takeover slot is part of this workflow.
  expect(sql(`SELECT to_regclass('public.media_discovery_execution_slot') IS NULL;
    SELECT count(*) FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace
    WHERE n.nspname = 'public' AND p.proname IN ('media_discovery_execution_claim_v1',
      'media_discovery_execution_renew_v1', 'media_discovery_execution_release_v1');`)).toBe('t\n0');
});
