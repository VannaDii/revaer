import { test, expect } from '../../fixtures/api';
import { apiFetchRaw } from '../../support/api/raw';
import { authHeaders } from '../../support/headers';

const MISSING_JOB_ID = '00000000-0000-0000-0000-000000000001';

const OPERATIONS = [
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
  ['PATCH', '/v1/media/profiles/{media_profile_public_id}'],
  ['PATCH', '/v1/media/profiles/{media_profile_public_id}/desired-target'],
  ['POST', '/v1/media/capabilities/refresh'],
  ['POST', '/v1/media/compatibility-targets'],
  ['POST', '/v1/media/discovery/preview'],
  ['POST', '/v1/media/discovery/runs'],
  ['POST', '/v1/media/discovery/schedules'],
  ['POST', '/v1/media/discovery/watchers'],
  ['POST', '/v1/media/imports/apply'],
  ['POST', '/v1/media/imports/validate'],
  ['POST', '/v1/media/jobs'],
  ['POST', '/v1/media/jobs/{media_job_public_id}/cancel'],
  ['POST', '/v1/media/jobs/{media_job_public_id}/phases'],
  ['POST', '/v1/media/jobs/{media_job_public_id}/retry'],
  ['POST', '/v1/media/planning/preview'],
  ['POST', '/v1/media/policies'],
  ['POST', '/v1/media/profiles'],
  ['POST', '/v1/media/profiles/validate'],
  ['POST', '/v1/media/targets'],
] as const;

test.describe('Media API', () => {
  test('routes bounded empty requests without server errors', async ({ baseUrl, session }) => {
    for (const [method, route] of OPERATIONS) {
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
});
