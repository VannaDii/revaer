import { test, expect } from '../../fixtures/media';
import { execFileSync } from 'node:child_process';

for (const removeParent of [false, true]) {
  test(`continues manual discovery after a candidate ${removeParent ? 'directory' : 'file'} is deleted`, async ({ api, mediaService, profileFixture: fixture }) => {
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
    const relativeMissing = `${fixture.prefix}/removed/source.mkv`;
    const folder = `/proof/source/${fixture.prefix}/removed`;
    const docker = (...args: string[]) => execFileSync('docker', ['exec', mediaService.container, ...args]);
    docker('mkdir', '-p', folder);
    docker('cp', fixture.sourcePath, `${folder}/source.mkv`);
    docker('rm', '-rf', '--', removeParent ? folder : `${folder}/source.mkv`);
    const admitted = await api.POST('/v1/media/discovery/runs', { body: {
      media_discovery_association_public_id: id,
      source_paths: [relativeMissing, fixture.relativePath],
    } });
    expect(admitted.response.status, JSON.stringify(admitted.error)).toBe(201);
    expect(admitted.data?.skipped).toEqual([{ source_path: relativeMissing, reason: 'media_discovery_source_unstable' }]);
    expect(admitted.data?.queued_jobs).toHaveLength(1);
    expect(admitted.data?.queued_jobs[0]).toMatchObject({ source_path: fixture.relativePath, dry_run: true });
    const jobId = admitted.data?.queued_jobs[0]?.media_job_public_id;
    if (!jobId) throw new Error('Remaining source was not admitted');
    const job = await api.GET('/v1/media/jobs/{media_job_public_id}', {
      params: { path: { media_job_public_id: jobId } },
    });
    expect(job.response.status, JSON.stringify(job.error)).toBe(200);
    expect(job.data).toMatchObject({ source_path: fixture.relativePath, dry_run: true });
    expect(fixture.readSource()).toEqual(fixture.sourceBytes);
  });
}
