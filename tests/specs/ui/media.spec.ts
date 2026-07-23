import { test, expect } from '../../fixtures/app';

test.describe('Media', () => {
  test.setTimeout(60_000);

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
    await expect(
      page.getByTestId('media-compliance-panel').getByText('redistributable-gplv3-runtime').first()
    ).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Compatibility targets' })).toBeVisible();
    await expect(
      page.getByTestId('media-target-catalog').getByText('hevc-aac', { exact: true })
    ).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Policies' })).toBeVisible();
    await expect(page.getByTestId('media-policy-catalog').getByText('safe_dry_run')).toBeVisible();
    const profileForm = page.getByTestId('media-profile-form');
    await expect(profileForm).toBeVisible();
    await expect(profileForm.getByLabel('compatibility_target_key')).toBeVisible();
    await expect(profileForm.getByLabel('policy_key')).toBeVisible();
    await expect(page.getByPlaceholder('schedule_interval_minutes')).toBeVisible();
    await expect(profileForm.getByText('Enable watcher')).toBeVisible();
    await expect(profileForm.getByText('Enable schedule')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Create profile' })).toBeVisible();

    const suffix = Date.now().toString(36);
    const targetKey = `ui-target-${suffix}`;
    const policyKey = `ui-policy-${suffix}`;
    const targetForm = page.getByTestId('media-target-form');
    await targetForm.getByPlaceholder('target_key').fill(targetKey);
    await targetForm.getByPlaceholder('target_version').fill('1');
    await targetForm.getByPlaceholder('target_display_name').fill(`UI target ${suffix}`);
    await targetForm.getByPlaceholder('target_video_codec').fill('hevc');
    await targetForm.getByPlaceholder('target_audio_codec').fill('aac');
    await targetForm.getByPlaceholder('target_audio_channels').fill('2');
    await targetForm.getByPlaceholder('target_audio_channel_layout').fill('stereo');
    await targetForm.getByLabel('target_subtitle_policy').selectOption('selected');
    const targetResponse = page.waitForResponse(
      (response) =>
        response.url().endsWith('/v1/media/compatibility-targets') &&
        response.request().method() === 'POST'
    );
    await targetForm.getByRole('button', { name: 'Save target' }).click();
    expect((await targetResponse).status()).toBe(201);
    await expect(page.getByRole('button', { name: 'Refresh', exact: true })).toBeDisabled();
    await expect(page.getByRole('button', { name: 'Refresh', exact: true })).toBeEnabled({
      timeout: 30_000,
    });
    await expect(
      page.getByTestId('media-target-catalog').getByText(targetKey, { exact: true })
    ).toBeVisible({ timeout: 10_000 });

    const policyForm = page.getByTestId('media-policy-form');
    await policyForm.getByPlaceholder('policy_catalog_key').fill(policyKey);
    await policyForm.getByPlaceholder('policy_version').fill('1');
    await policyForm.getByPlaceholder('policy_display_name').fill(`UI policy ${suffix}`);
    await policyForm.getByLabel('policy_video_intent').selectOption('general');
    const policyResponse = page.waitForResponse(
      (response) =>
        response.url().endsWith('/v1/media/policies') && response.request().method() === 'POST'
    );
    await policyForm.getByRole('button', { name: 'Save policy' }).click();
    expect((await policyResponse).status()).toBe(201);
    await expect(page.getByRole('button', { name: 'Refresh', exact: true })).toBeDisabled();
    await expect(page.getByRole('button', { name: 'Refresh', exact: true })).toBeEnabled({
      timeout: 30_000,
    });
    await expect(
      page.getByTestId('media-policy-catalog').getByText(policyKey, { exact: true })
    ).toBeVisible({ timeout: 10_000 });

    await profileForm.getByLabel('compatibility_target_key').selectOption(targetKey);
    await expect(profileForm.getByLabel('compatibility_target_key')).toHaveValue(targetKey);
    await profileForm.getByLabel('policy_key').selectOption(policyKey);
    await expect(profileForm.getByLabel('policy_key')).toHaveValue(policyKey);

    await expect(page.getByRole('heading', { name: 'YAML import/export' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Validate YAML' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Apply YAML' })).toBeVisible();
  });
});
