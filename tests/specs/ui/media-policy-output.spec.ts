import { test, expect } from '../../fixtures/app';

// Uses the real authenticated service; it does not qualify media execution.
test('authors complete output intent and preserves a rejected draft', async ({ app, page }, testInfo) => {
  const key = `ui-output-${crypto.randomUUID()}`;
  await app.goto('/media');
  const form = page.getByTestId('media-policy-form');
  await form.getByPlaceholder('policy_catalog_key').fill(key);
  await form.getByPlaceholder('policy_version').fill('1');
  await form.getByPlaceholder('policy_display_name').fill('UI output policy');
  for (const label of ['Policy dry-run', 'Quarantine failures', 'Preserve permissions', 'Preserve ownership']) {
    await expect(form.getByLabel(label, { exact: true })).toBeChecked();
  }
  await expect(form.getByLabel('Replacement mode', { exact: true })).toHaveValue('disabled');
  await form.getByLabel('Policy dry-run', { exact: true }).uncheck();
  const response = () => page.waitForResponse(result =>
    result.request().method() === 'POST' && new URL(result.url()).pathname === '/v1/media/policies');
  const rejected = response();
  await form.getByRole('button', { name: 'Save policy', exact: true }).click();
  expect((await rejected).status()).toBe(400);
  await expect(form.getByPlaceholder('policy_catalog_key')).toHaveValue(key);
  await expect(form.getByLabel('Policy dry-run', { exact: true })).not.toBeChecked();
  await form.getByLabel('Replacement mode', { exact: true }).selectOption('atomic_replace');
  await form.getByLabel('Quarantine failures', { exact: true }).uncheck();
  await form.getByLabel('Preserve permissions', { exact: true }).uncheck();
  const created = response();
  await form.getByRole('button', { name: 'Save policy', exact: true }).click();
  const saved = await created;
  expect(saved.status(), await saved.text()).toBe(201);
  expect((await saved.json()).output).toEqual({
    dry_run: false, replacement_mode: 'atomic_replace', quarantine_enabled: false,
    preserve_permissions: false, preserve_ownership: true,
  });
  const catalog = page.getByTestId('media-policy-catalog');
  await expect(catalog).toContainText(key);
  await expect(catalog).toContainText('Execution | Replacement: atomic_replace | Quarantine: false | Permissions: false | Ownership: true');
  for (const width of [1280, 390]) {
    await page.setViewportSize({ width, height: 900 });
    expect(await form.evaluate(element => element.scrollWidth <= element.clientWidth)).toBe(true);
    await testInfo.attach(`output-policy-${width}`, {
      body: await form.screenshot({ animations: 'disabled' }), contentType: 'image/png',
    });
  }
});
