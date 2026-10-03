import { test, expect } from '../../fixtures/app';

// Route-controlled state/race coverage; real retry evidence is recorded separately.
test('job actions require confirmation and refresh after rejected and accepted requests', async ({ app, page }) => {
  const id = '00000000-0000-0000-0000-000000000055';
  let status = 'failed';
  let requests = 0;
  let release: (() => void) | undefined;
  await page.route(url => url.pathname === '/v1/media/jobs/recent', route => route.fulfill({ json: {
    jobs: [{ media_job_public_id: id, media_profile_public_id: id,
      source_path: 'Movies/a.mkv', output_path: 'Movies/a.mkv', status, dry_run: false,
      queued_at: '2026-10-02T00:00:00Z',
      diagnostic_counts: { operations: 0, violations: 0, plan_reasons: 0,
        verification_checks: 0, artifacts: 0, compact_audits: 0 } }],
  } }));
  await page.route(`**/v1/media/jobs/${id}/diagnostics`, route => route.fulfill({ json: {
    media_job_public_id: id, operations: [], violations: [], plan_reasons: [],
    verification_checks: [], artifacts: [], compact_audits: [],
  } }));
  await page.route(`**/v1/media/jobs/${id}/retry`, async route => {
    requests += 1;
    if (requests === 1) {
      await new Promise<void>(resolve => { release = resolve; });
      return route.fulfill({ status: 409, json: { title: 'Conflict', status: 409 } });
    }
    status = 'queued';
    return route.fulfill({ status: 204 });
  });
  await page.route(`**/v1/media/jobs/${id}/cancel`, route => {
    requests += 1;
    status = 'cancelled';
    return route.fulfill({ status: 204 });
  });
  await app.goto('/media');
  const row = page.locator(`[data-job-id="${id}"]`);
  await row.locator('summary').click();
  const retry = row.getByRole('button', { name: 'Retry job', exact: true });
  await expect(retry).toBeDisabled();
  await row.getByLabel('Confirm retry', { exact: true }).check();
  await retry.click();
  await expect.poll(() => requests).toBe(1);
  await expect(retry).toBeDisabled();
  await expect(row.getByLabel('Confirm retry', { exact: true })).not.toBeChecked();
  release?.();
  await expect(row.getByRole('status')).toContainText('Job state changed.');
  await expect(retry).toBeDisabled();
  await row.getByLabel('Confirm retry', { exact: true }).check();
  await retry.click();
  const cancel = row.getByRole('button', { name: 'Cancel job', exact: true });
  await expect(cancel).toBeDisabled();
  await row.getByLabel('Confirm cancellation', { exact: true }).check();
  await cancel.click();
  await expect(retry).toBeDisabled();
  expect(requests).toBe(3);
});
