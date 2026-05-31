import { test, expect } from '../../fixtures/app';

test.describe('Media', () => {
  test('renders media management surface', async ({ app, page }) => {
    await app.goto('/media');

    await expect(page.getByRole('heading', { name: 'Media' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Refresh', exact: true })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Refresh capability' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Export YAML' })).toBeVisible();
    await expect(page.getByTestId('media-profile-form')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Create profile' })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'YAML import/export' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Validate YAML' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Apply YAML' })).toBeVisible();
  });
});
