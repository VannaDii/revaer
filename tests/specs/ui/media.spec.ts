import { test, expect } from '../../fixtures/app';

test.describe('Media', () => {
  test('renders media management surface', async ({ app, page }) => {
    await app.goto('/media');

    await expect(page.getByRole('heading', { name: 'Media' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Refresh', exact: true })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Refresh capability' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Export YAML' })).toBeVisible();
    await expect(page.getByText('License mode')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Refresh', exact: true })).toBeEnabled({
      timeout: 20_000,
    });
    await expect(page.getByText('redistributable-gplv3-runtime')).toBeVisible();
    const profileForm = page.getByTestId('media-profile-form');
    await expect(profileForm).toBeVisible();
    await expect(page.getByPlaceholder('compatibility_target_key')).toBeVisible();
    await expect(page.getByPlaceholder('policy_key')).toBeVisible();
    await expect(page.getByPlaceholder('schedule_interval_minutes')).toBeVisible();
    await expect(profileForm.getByText('Enable watcher')).toBeVisible();
    await expect(profileForm.getByText('Enable schedule')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Create profile' })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'YAML import/export' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Validate YAML' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Apply YAML' })).toBeVisible();
  });
});
