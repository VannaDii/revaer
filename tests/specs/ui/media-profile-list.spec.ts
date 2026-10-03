import { test, expect } from '../../fixtures/app';

// Route-controlled frontend evidence only; no persistence or restart is simulated as proof.
const id = '00000000-0000-0000-0000-000000000011';
const collection = (url: URL) => url.pathname === '/v1/media/profiles';

function profile(key = 'movies') {
  return {
    media_profile_public_id: key === 'tv' ? '00000000-0000-0000-0000-000000000012' : id,
    profile_key: key, display_name: ' Exact name ', description: '',
    enabled: false, dry_run_only: true, desired_target_key: 'archive', desired_target_version: 2,
    policy_key: 'preserve', policy_version: 3, output_root_key: 'output', workspace_root_key: 'workspace',
    latest_version: 1, active_version: 1, lifecycle_state: 'active',
    created_at: '2026-10-01T00:00:00Z', updated_at: '2026-10-01T00:00:00Z',
    root_bindings: ['output', 'workspace'].map(kind => ({
      kind, logical_key: kind, resolution_state: 'resolved', binding_ready: true,
      destructive_ready: false, destructive_reason: 'media_root_durability_unproven',
    })),
  };
}

function cursor(key: string) {
  const length = Buffer.alloc(2);
  length.writeUInt16BE(Buffer.byteLength(key));
  return `profiles_${Buffer.concat([length, Buffer.from(key), Buffer.from(id.replaceAll('-', ''), 'hex')]).toString('base64url')}`;
}

function catalog() {
  return {
    format_version: 1, source_state: 'ready', attestation_state: 'ready',
    generation: {
      media_root_catalog_generation_public_id: '00000000-0000-0000-0000-000000000033',
      generation: '1', source_sha256: 'a'.repeat(64), generation_sha256: 'b'.repeat(64),
      slot_count: 2, activated_at: '2026-10-01T00:00:00Z', reconciled_at: '2026-10-01T00:00:00Z',
    },
    slots: ['output', 'workspace'].map((kind, index) => ({
      media_root_catalog_slot_public_id: `00000000-0000-0000-0000-00000000000${index + 1}`,
      logical_key: kind, requested_path: `/fixture/${kind}`, canonical_path: `/fixture/${kind}`,
      filesystem_device: '0000000000000001', filesystem_inode: `000000000000000${index + 2}`,
      mount_id: '1', filesystem_type: 'ext4', read_capable: true, write_capable: true,
      create_new_capable: true, fsync_capable: true, rename_capable: true,
      delete_capable: true, capacity_probe_capable: true,
      durability_class: 'disposable', durability_evidence: 'none',
      sole_writer_class: 'revaer_exclusive', sole_writer_evidence: 'linux_dedicated_service',
      owner_uid: 1000, owner_gid: 1000, mode_bits: '0700', validated_at: '2026-10-01T00:00:00Z',
      root_identity_sha256: (index + 1).toString(16).padStart(64, '0'), allowed_kinds: [{ kind, binding_ready: true,
        destructive_ready: false, destructive_reason: 'media_root_durability_unproven' }],
    })),
  };
}

test.describe('Versioned profile dashboard', () => {
  test('paginates complete path-free profiles and clears evidence on failure', async ({ app, page }, testInfo) => {
    let status = 200;
    const continuation = cursor('movies');
    await page.route(collection, route => {
      const url = new URL(route.request().url());
      if (url.searchParams.get('limit') === '200') return route.fulfill({ json: { profiles: [] } });
      expect(url.searchParams.get('limit')).toBe('50');
      return route.fulfill({ status, json: status !== 200
        ? { title: 'Forbidden', detail: '/private/never-display', status }
        : url.searchParams.has('cursor')
          ? { profiles: [profile('tv')] }
          : { profiles: [{ ...profile(), display_name: 'N'.repeat(128), description: 'D'.repeat(1024),
            desired_target_key: 't'.repeat(64) }], next_cursor: continuation } });
    });
    await app.goto('/media');
    const panel = page.getByTestId('media-profile-list');
    await expect(panel).toContainText('movies | Active | Disabled | Dry-run only');
    await expect(panel).toContainText('Output: output | binding ready | destructive not ready');
    await expect(page.getByTestId('media-profile-form')).toHaveCount(0);
    for (const width of [1280, 390]) {
      await page.setViewportSize({ width, height: 844 });
      const bounds = await panel.boundingBox();
      expect(bounds).not.toBeNull();
      expect((bounds?.x ?? width) + (bounds?.width ?? width)).toBeLessThanOrEqual(width);
      expect(await panel.evaluate(element => element.scrollWidth <= element.clientWidth)).toBe(true);
      const screenshot = testInfo.outputPath(`profile-list-${width}.png`);
      await panel.screenshot({ path: screenshot, animations: 'disabled' });
      await testInfo.attach(`profile-list-${width}`, { path: screenshot, contentType: 'image/png' });
    }
    await panel.getByRole('button', { name: 'Next profiles', exact: true }).click();
    await expect(panel).toContainText('tv | Active');
    await expect(panel.getByRole('button', { name: 'Next profiles', exact: true })).toBeDisabled();
    await panel.getByRole('button', { name: 'Previous profiles', exact: true }).click();
    await expect(panel).toContainText('movies | Active');
    status = 403;
    await panel.getByRole('button', { name: 'Reload profiles', exact: true }).click();
    await expect(panel.getByRole('alert')).toHaveText('Profiles could not be loaded.');
    await expect(panel.getByTestId('media-profile-summary')).toHaveCount(0);
    await expect(panel).not.toContainText('/private');
  });

  test('confirmed complete save refreshes the list without clearing association intent', async ({ app, page }) => {
    let saved: ReturnType<typeof profile> | undefined;
    let submissions = 0;
    await page.route(url => url.pathname === '/v1/media/root-catalog' && url.searchParams.get('limit') === '200',
      route => route.fulfill({ json: catalog() }));
    await page.route(collection, async route => {
      if (route.request().method() === 'POST') {
        submissions += 1;
        expect(route.request().headers()['if-none-match']).toBe('*');
        const request = route.request().postDataJSON();
        expect(request).toEqual({
          profile_key: 'movies', display_name: ' Exact name ', description: '', enabled: false,
          dry_run_only: true, desired_target_key: 'archive', desired_target_version: 2,
          policy_key: 'preserve', policy_version: 3, output_root_key: 'output', workspace_root_key: 'workspace',
        });
        saved = { ...profile(), ...request };
        await route.fulfill({ status: 201, headers: {
          ETag: `"media-profile:${id}:v1"`, 'Access-Control-Expose-Headers': 'ETag',
        }, json: saved });
      } else {
        await route.fulfill({ json: { profiles: saved ? [saved] : [] } });
      }
    });
    await app.goto('/media');
    const association = page.getByTestId('media-association-editor');
    await association.getByRole('textbox', { name: 'Association key' }).fill('retain-my-intent');
    const editor = page.getByTestId('media-profile-root-editor');
    for (const [label, value] of [
      ['Profile key', 'movies'], ['Display name', ' Exact name '], ['Desired target key', 'archive'],
      ['Desired target version', '2'], ['Policy key', 'preserve'], ['Policy version', '3'],
    ]) await editor.getByLabel(label, { exact: true }).fill(value);
    await editor.getByRole('combobox', { name: 'Output root', exact: true }).selectOption('output');
    await editor.getByRole('combobox', { name: 'Workspace root', exact: true }).selectOption('workspace');
    await editor.getByRole('button', { name: 'Create profile', exact: true }).click();
    await expect(editor.getByRole('status').filter({ hasText: 'Profile saved as version 1.' }))
      .toHaveText('Profile saved as version 1.');
    await expect(page.getByTestId('media-profile-list')).toContainText('movies | Active | Disabled | Dry-run only');
    await expect(association.getByRole('textbox', { name: 'Association key' })).toHaveValue('retain-my-intent');
    expect(submissions).toBe(1);
  });
});
