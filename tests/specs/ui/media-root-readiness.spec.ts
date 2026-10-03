import { test, expect } from '../../fixtures/app';

// Transport-controlled UI regressions, not filesystem or service readiness proof.
const readinessPath = '**/v1/media/root-catalog/readiness';
const kinds = ['source', 'output', 'workspace', 'backup', 'quarantine'];

function readyCatalog(count: number) {
  return {
    format_version: 1,
    source_state: 'ready',
    attestation_state: 'ready',
    generation: '1',
    kinds: kinds.map((kind) => ({
      kind,
      attested_slot_count: count,
      binding_ready_slot_count: count,
      destructive_ready_slot_count: 0,
    })),
  };
}

test.describe('Root readiness presentation', () => {
  test('reports unavailable endpoint and refreshes into an empty catalog', async ({ app, page }) => {
    let status = 404;
    await page.route(readinessPath, (route) => route.fulfill({
      status,
      json: status === 200 ? readyCatalog(0) : {
        title: 'Not found', detail: '/private/root/must-not-appear', status,
      },
    }));
    await app.goto('/media');
    const panel = page.getByTestId('media-root-readiness');
    await expect(panel.getByRole('alert')).toHaveText('Root readiness is unavailable on this server.');
    await expect(panel).not.toContainText('/private/root');
    status = 200;
    await panel.getByRole('button', { name: 'Refresh root readiness' }).click();
    await expect(panel.getByRole('status')).toHaveText('The catalog contains no attested root slots.');
    await expect(panel.getByRole('row')).toHaveCount(6);
  });

  test('keeps binding and destructive readiness distinct on desktop and mobile', async ({ app, page }, testInfo) => {
    await page.route(readinessPath, (route) => route.fulfill({ json: readyCatalog(2) }));
    await app.goto('/media');
    const panel = page.getByTestId('media-root-readiness');
    for (const width of [1280, 390]) {
      await page.setViewportSize({ width, height: 844 });
      await expect(panel.getByRole('columnheader', { name: 'Binding ready', exact: true })).toBeVisible();
      await expect(panel.getByRole('columnheader', { name: 'Destructive ready', exact: true })).toBeVisible();
      const destructiveHeader = await panel.getByRole('columnheader', { name: 'Destructive ready', exact: true }).boundingBox();
      expect(destructiveHeader).not.toBeNull();
      expect((destructiveHeader?.x ?? width) + (destructiveHeader?.width ?? width)).toBeLessThanOrEqual(width);
      const headerTextFits = await panel.getByRole('columnheader', { name: 'Destructive ready', exact: true }).evaluate((header) => {
        const range = header.ownerDocument.createRange();
        range.selectNodeContents(header);
        return range.getBoundingClientRect().right <= header.getBoundingClientRect().right;
      });
      expect(headerTextFits).toBe(true);
      const refreshIcon = panel.getByRole('button', { name: 'Refresh root readiness' }).locator('svg');
      await expect(refreshIcon).toBeVisible();
      const iconBounds = await refreshIcon.boundingBox();
      expect(iconBounds?.width).toBeGreaterThan(0);
      expect(iconBounds?.width).toBeLessThanOrEqual(32);
      for (const kind of kinds) {
        const label = kind.charAt(0).toUpperCase() + kind.slice(1);
        const row = panel.getByRole('row').filter({ has: page.getByRole('rowheader', { name: label, exact: true }) });
        await expect(row.getByRole('cell')).toHaveText(['2', '2', '0']);
      }
      const bounds = await panel.boundingBox();
      expect(bounds).not.toBeNull();
      expect((bounds?.x ?? width) + (bounds?.width ?? width)).toBeLessThanOrEqual(width);
      await testInfo.attach(`root-readiness-${width}`, {
        body: await panel.screenshot({ animations: 'disabled' }), contentType: 'image/png',
      });
    }
  });

  test('does not retain ready counts after access is denied', async ({ app, page }) => {
    let status = 200;
    await page.route(readinessPath, (route) => route.fulfill({
      status,
      json: status === 200 ? readyCatalog(2) : { title: 'Forbidden', status },
    }));
    await app.goto('/media');
    const panel = page.getByTestId('media-root-readiness');
    await expect(panel.getByRole('table')).toBeVisible();
    status = 403;
    await panel.getByRole('button', { name: 'Refresh root readiness' }).click();
    await expect(panel.getByRole('alert')).toHaveText('Access to root readiness was denied.');
    await expect(panel.getByRole('table')).toHaveCount(0);
  });
});
