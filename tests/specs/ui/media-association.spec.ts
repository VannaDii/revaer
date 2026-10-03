import { test, expect } from '../../fixtures/app';

// These route-controlled tests verify UI contracts, not root attestation or persistence.
const profileId = '00000000-0000-0000-0000-000000000011';
const associationId = '00000000-0000-0000-0000-000000000022';
const requestedPath = '/private/catalog-source/library';
const canonicalPath = '/private/catalog-canonical/library';
const catalogRoute = (url: URL) => url.pathname === '/v1/media/root-catalog' && url.searchParams.get('limit') === '200';
const profilesRoute = (url: URL) => url.pathname === '/v1/media/profiles' && url.searchParams.get('limit') === '200';

function catalog() {
  return {
    format_version: 1, source_state: 'ready', attestation_state: 'ready',
    generation: {
      media_root_catalog_generation_public_id: '00000000-0000-0000-0000-000000000033',
      generation: '1', source_sha256: 'a'.repeat(64), generation_sha256: 'b'.repeat(64),
      slot_count: 1, activated_at: '2026-09-15T00:00:00Z', reconciled_at: '2026-09-15T00:00:00Z',
    },
    slots: [{
      media_root_catalog_slot_public_id: '00000000-0000-0000-0000-000000000044',
      logical_key: 'library', requested_path: requestedPath, canonical_path: canonicalPath,
      filesystem_device: '0000000000000001', filesystem_inode: '0000000000000002',
      mount_id: '1', filesystem_type: 'ext4', read_capable: true, write_capable: true,
      create_new_capable: true, fsync_capable: true, rename_capable: true,
      delete_capable: true, capacity_probe_capable: true,
      durability_class: 'disposable', durability_evidence: 'none',
      sole_writer_class: 'revaer_exclusive', sole_writer_evidence: 'linux_dedicated_service',
      owner_uid: 1000, owner_gid: 1000, mode_bits: '0700',
      validated_at: '2026-09-15T00:00:00Z', root_identity_sha256: 'c'.repeat(64),
      allowed_kinds: [{ kind: 'source', binding_ready: true, destructive_ready: false,
        destructive_reason: 'media_root_durability_unproven' }],
    }],
  };
}

test.describe('Catalog-backed media configuration', () => {
  test.beforeEach(async ({ page }) => {
    await page.route(catalogRoute, route => route.fulfill({ json: catalog() }));
    await page.route(profilesRoute, route => route.fulfill({ json: {
      profiles: [{ media_profile_public_id: profileId, profile_key: 'balanced', latest_version: 3, active_version: 3 }],
    } }));
  });

  test('previews bounded relative candidates and requires explicit queue confirmation', async ({ app, page }, testInfo) => {
    let previews = 0;
    let runs = 0;
    let saved: Record<string, unknown> | undefined;
    let collectionStatus = 200;
    await page.route(url => url.pathname === '/v1/media/discovery-associations', route => {
      if (route.request().method() === 'GET') return route.fulfill({ status: collectionStatus, json: collectionStatus === 200
        ? { associations: saved ? [saved] : [] } : { status: collectionStatus, title: 'Forbidden', detail: '/private/do-not-display' } });
      saved = { ...route.request().postDataJSON(), media_discovery_association_public_id: associationId,
        latest_version: 1, active_version: 1, lifecycle_state: 'active', resolution_state: 'resolved',
        binding_ready: true, destructive_ready: false, destructive_reason: 'media_root_durability_unproven',
        created_at: '2026-10-02T00:00:00Z' };
      return route.fulfill({ status: 201, headers: { ETag: '"association-created-v1"', 'Access-Control-Expose-Headers': 'ETag' }, json: saved });
    });
    await page.route('**/v1/media/discovery/preview', route => {
      previews += 1;
      expect(route.request().postDataJSON()).toEqual({
        media_discovery_association_public_id: associationId, source_paths: ['Movies/a.mkv', 'Other/b.mkv'],
      });
      return route.fulfill({ json: { previews: [
        { source_path: 'Movies/a.mkv', output_path: 'Movies/a.mkv', accepted: true, dry_run: true },
        { source_path: 'Other/b.mkv', accepted: false, dry_run: true, reason: 'media_discovery_source_path_outside_profile_root' },
      ] } });
    });
    await page.route('**/v1/media/discovery/runs', route => {
      runs += 1;
      expect(route.request().postDataJSON()).toEqual({
        media_discovery_association_public_id: associationId, source_paths: ['Movies/a.mkv', 'Other/b.mkv'],
      });
      if (runs === 1) return route.fulfill({ status: 503, json: { status: 503, title: 'Unavailable' } });
      return route.fulfill({ status: 201, json: {
        queued_jobs: [{ media_job_public_id: '00000000-0000-0000-0000-000000000055',
          source_path: 'Movies/a.mkv', output_path: 'Movies/a.mkv', dry_run: true }],
        skipped: [{ source_path: 'Other/b.mkv', reason: 'media_discovery_source_path_outside_profile_root' }],
      } });
    });
    await app.goto('/media');
    const editor = page.getByTestId('media-association-editor');
    await editor.getByLabel('Association key', { exact: true }).fill('manual-library');
    await editor.getByLabel('Active profile version').selectOption(profileId);
    await editor.getByRole('combobox', { name: 'Source root', exact: true }).selectOption('library');
    await editor.getByLabel('Whole root', { exact: true }).check();
    await editor.getByLabel('Manual', { exact: true }).check();
    await editor.getByRole('button', { name: 'Create association', exact: true }).click();
    const manual = page.getByTestId('media-manual-discovery');
    await expect(manual).toBeVisible();
    await manual.getByLabel('Relative candidates').fill('/private/a');
    await manual.getByRole('button', { name: 'Preview candidates' }).click();
    await expect(manual.getByRole('alert')).toContainText('root-relative');
    expect(previews).toBe(0);
    await manual.getByLabel('Relative candidates').fill('Movies/a.mkv\nOther/b.mkv');
    await expect(manual.getByRole('button', { name: 'Queue jobs', exact: true })).toBeDisabled();
    await manual.getByRole('button', { name: 'Preview candidates' }).click();
    await expect(manual).toContainText('Movies/a.mkv | Accepted | Dry-run');
    await expect(manual).toContainText('Other/b.mkv | Rejected | Dry-run');
    for (const width of [1280, 390]) {
      await page.setViewportSize({ width, height: 844 });
      expect(await manual.evaluate(element => element.scrollWidth <= element.clientWidth)).toBe(true);
      await testInfo.attach(`manual-discovery-${width}`, {
        body: await manual.screenshot({ animations: 'disabled' }), contentType: 'image/png',
      });
    }
    await expect(manual.getByRole('button', { name: 'Queue jobs', exact: true })).toBeDisabled();
    await manual.getByLabel('Queue jobs using the active profile').check();
    await manual.getByRole('button', { name: 'Queue jobs', exact: true }).click();
    await expect(manual.getByRole('alert')).toContainText('Check recent jobs before resubmitting.');
    await expect(manual.getByLabel('Relative candidates')).toHaveValue('Movies/a.mkv\nOther/b.mkv');
    await expect(manual.getByLabel('Queue jobs using the active profile')).not.toBeChecked();
    expect(runs).toBe(1);
    await manual.getByLabel('Queue jobs using the active profile').check();
    await manual.getByRole('button', { name: 'Queue jobs', exact: true }).click();
    await expect(manual.getByRole('status')).toHaveText('1 queued; 1 skipped.');
    await expect(manual.getByRole('button', { name: 'Queue jobs', exact: true })).toBeDisabled();
    expect(runs).toBe(2);
    await manual.getByLabel('Relative candidates').fill('Movies/new.mkv');
    await expect(manual).not.toContainText('1 queued; 1 skipped.');
    await expect(manual.getByLabel('Queue jobs using the active profile')).not.toBeChecked();
    await expect(manual.getByRole('button', { name: 'Queue jobs', exact: true })).toBeDisabled();
    await page.reload();
    const selector = page.getByTestId('media-association-list').getByRole('combobox', { name: 'Manual association' });
    await expect(selector.locator('option')).toHaveCount(2);
    await expect(manual).toHaveCount(0);
    await selector.selectOption(associationId);
    await expect(manual).toBeVisible();
    await expect(manual.getByLabel('Relative candidates')).toHaveValue('');
    expect(runs).toBe(2);
    collectionStatus = 403;
    const list = page.getByTestId('media-association-list');
    await list.getByRole('button', { name: 'Reload associations' }).click();
    await expect(list.getByRole('alert')).toHaveText('Discovery associations could not be loaded.');
    await expect(manual).toHaveCount(0);
    await expect(list).not.toContainText('/private');
  });

  test('submits an explicit whole-root logical association with a create precondition', async ({ app, page }, testInfo) => {
    let submissions = 0;
    await page.route('**/v1/media/discovery-associations', async route => {
      submissions += 1;
      expect(route.request().headers()['if-none-match']).toBe('*');
      expect(route.request().postDataJSON()).toEqual({
        association_key: 'scan-library', media_profile_public_id: profileId, profile_version: 3,
        source_root_key: 'library', root_relative_path: '', manual_enabled: true,
        watcher_enabled: false, schedule_enabled: false,
      });
      await route.fulfill({ status: 201, headers: { ETag: '"association-created-v1"', 'Access-Control-Expose-Headers': 'ETag' }, json: {
        ...route.request().postDataJSON(),
        media_discovery_association_public_id: associationId, association_key: 'scan-library',
        latest_version: 1, active_version: 1,
      } });
    });
    await app.goto('/media');
    const admin = page.getByTestId('media-root-catalog');
    await admin.getByText('library', { exact: true }).click();
    await expect(admin).toContainText(requestedPath);
    await expect(admin).toContainText(canonicalPath);
    const editor = page.getByTestId('media-association-editor');
    await expect(editor).not.toContainText(requestedPath);
    await expect(editor).not.toContainText(canonicalPath);
    await editor.getByRole('textbox', { name: 'Association key' }).fill('scan-library');
    await editor.getByLabel('Active profile version').selectOption(profileId);
    await editor.getByRole('combobox', { name: 'Source root', exact: true }).selectOption('library');
    await editor.getByRole('button', { name: 'Create association', exact: true }).click();
    await expect(editor.getByRole('alert')).toHaveText('Select whole-root or relative-prefix scope.');
    expect(submissions).toBe(0);
    await editor.getByLabel('Whole root', { exact: true }).check();
    await editor.getByLabel('Manual', { exact: true }).check();
    for (const width of [1280, 390]) {
      await page.setViewportSize({ width, height: 844 });
      const bounds = await editor.boundingBox();
      expect(bounds).not.toBeNull();
      expect((bounds?.x ?? width) + (bounds?.width ?? width)).toBeLessThanOrEqual(width);
      await testInfo.attach(`association-${width}`, {
        body: await editor.screenshot({ animations: 'disabled' }), contentType: 'image/png',
      });
    }
    await editor.getByRole('button', { name: 'Create association', exact: true }).click();
    await expect(editor.getByRole('status')).toContainText('active version 1');
    expect(submissions).toBe(1);
    await expect(editor.getByRole('button', { name: 'Create association', exact: true })).toBeDisabled();
  });

  test('preserves prefix and mode intent on conflict without an automatic mutation retry', async ({ app, page }) => {
    let submissions = 0;
    await page.route('**/v1/media/discovery-associations', async route => {
      submissions += 1;
      expect(route.request().postDataJSON()).toMatchObject({
        root_relative_path: 'Series/Season 1', watcher_enabled: true, schedule_enabled: true,
      });
      await route.fulfill({ status: 412, json: { title: 'Conflict', status: 412, detail: requestedPath } });
    });
    await app.goto('/media');
    const editor = page.getByTestId('media-association-editor');
    await editor.getByRole('textbox', { name: 'Association key' }).fill('scan-series');
    await editor.getByLabel('Active profile version').selectOption(profileId);
    await editor.getByRole('combobox', { name: 'Source root', exact: true }).selectOption('library');
    await editor.getByRole('radio', { name: 'Relative prefix', exact: true }).check();
    await editor.getByRole('textbox', { name: 'Relative prefix', exact: true }).fill('Series/Season 1');
    await editor.getByLabel('Watcher', { exact: true }).check();
    await editor.getByLabel('Schedule', { exact: true }).check();
    await editor.getByRole('button', { name: 'Create association', exact: true }).click();
    await expect(editor.getByRole('alert')).toContainText('preserved draft');
    await expect(editor).not.toContainText(requestedPath);
    await expect(editor.getByRole('textbox', { name: 'Relative prefix', exact: true })).toHaveValue('Series/Season 1');
    await expect(editor.getByLabel('Watcher', { exact: true })).toBeChecked();
    await expect(editor.getByLabel('Schedule', { exact: true })).toBeChecked();
    expect(submissions).toBe(1);
  });

  test('withholds root choices when the catalog claims missing slots', async ({ app, page }) => {
    const incomplete = catalog();
    incomplete.generation.slot_count = 2;
    await page.route(catalogRoute, route => route.fulfill({ json: incomplete }));
    await app.goto('/media');
    await expect(page.getByTestId('media-root-catalog').getByRole('alert')).toBeVisible();
    const editor = page.getByTestId('media-association-editor');
    await expect(editor.getByRole('combobox', { name: 'Source root', exact: true }).locator('option')).toHaveCount(1);
    await expect(editor.getByRole('button', { name: 'Create association', exact: true })).toBeDisabled();
  });

  test('keeps profile root drafts kind-scoped and preserves selections after catalog access is lost', async ({ app, page }, testInfo) => {
    await app.goto('/media');
    const editor = page.getByTestId('media-profile-root-editor');
    await expect(editor.getByRole('combobox', { name: 'Output root', exact: true }).locator('option')).toHaveCount(1);
    await expect(editor.getByLabel('Enabled', { exact: true })).not.toBeChecked();
    await expect(editor.getByLabel('Dry-run only', { exact: true })).toBeChecked();
    await expect(editor.getByLabel('Desired target version', { exact: true })).toHaveValue('');
    await expect(editor.getByLabel('Policy version', { exact: true })).toHaveValue('');
    const available = catalog();
    available.generation.slot_count = 4;
    available.slots = ['backup', 'output', 'quarantine', 'workspace'].map((kind, index) => ({
      ...available.slots[0],
      media_root_catalog_slot_public_id: `00000000-0000-0000-0000-00000000005${index}`,
      logical_key: kind,
      requested_path: `${requestedPath}/${kind}`,
      canonical_path: `${canonicalPath}/${kind}`,
      filesystem_inode: `000000000000000${index + 3}`,
      root_identity_sha256: String(index + 1).repeat(64),
      allowed_kinds: [{ kind, binding_ready: true, destructive_ready: false,
        destructive_reason: 'media_root_durability_unproven' }],
    }));
    await page.route(catalogRoute, route => route.fulfill({ json: available }));
    await page.getByRole('button', { name: 'Reload root catalog' }).click();
    for (const kind of ['Output', 'Workspace', 'Backup', 'Quarantine']) {
      const field = editor.getByRole('combobox', { name: `${kind} root`, exact: true });
      await expect(field.locator('option')).toHaveCount(2);
      await expect(field).toHaveValue('');
      await field.selectOption(kind.toLowerCase());
    }
    await expect(editor.getByText('Binding ready. Destructive operations are not ready.', { exact: true })).toHaveCount(4);
    await expect(editor).not.toContainText(requestedPath);
    await expect(editor).not.toContainText(canonicalPath);
    await editor.getByLabel('Profile key', { exact: true }).fill('library-profile');
    await editor.getByLabel('Display name', { exact: true }).fill('Library profile');
    await editor.getByLabel('Desired target key', { exact: true }).fill('archive');
    await editor.getByLabel('Desired target version', { exact: true }).fill('2');
    await editor.getByLabel('Policy key', { exact: true }).fill('preserve');
    await editor.getByLabel('Policy version', { exact: true }).fill('3');
    let profileSubmissions = 0;
    await page.route('**/v1/media/profiles', async route => {
      if (route.request().method() !== 'POST') { await route.continue(); return; }
      profileSubmissions += 1;
      expect(route.request().headers()['if-none-match']).toBe('*');
      expect(route.request().postDataJSON()).toEqual({
        profile_key: 'library-profile', display_name: 'Library profile', description: '',
        enabled: false, dry_run_only: true,
        desired_target_key: 'archive', desired_target_version: 2, policy_key: 'preserve', policy_version: 3,
        output_root_key: 'output', workspace_root_key: 'workspace',
        backup_root_key: 'backup', quarantine_root_key: 'quarantine',
      });
      await route.fulfill({ status: 412, json: { title: 'Conflict', status: 412 } });
    });
    await editor.getByRole('button', { name: 'Create profile', exact: true }).click();
    await expect(editor.getByRole('alert')).toContainText('preserved draft');
    await expect(editor.getByLabel('Profile key', { exact: true })).toHaveValue('library-profile');
    expect(profileSubmissions).toBe(1);
    for (const width of [1280, 390]) {
      await page.setViewportSize({ width, height: 844 });
      for (const field of await editor.getByRole('combobox').all()) {
        const bounds = await field.boundingBox();
        expect(bounds).not.toBeNull();
        expect((bounds?.x ?? width) + (bounds?.width ?? width)).toBeLessThanOrEqual(width);
      }
      await testInfo.attach(`profile-roots-${width}`, {
        body: await editor.screenshot({ animations: 'disabled' }), contentType: 'image/png',
      });
    }
    await page.route(catalogRoute, route => route.fulfill({ status: 403, json: { title: 'Denied', status: 403 } }));
    await page.getByRole('button', { name: 'Reload root catalog' }).click();
    await expect(page.getByTestId('media-root-catalog').getByRole('alert')).toHaveText('Access to the root catalog was denied.');
    for (const kind of ['Output', 'Workspace', 'Backup', 'Quarantine']) {
      await expect(editor.getByRole('combobox', { name: `${kind} root`, exact: true })).toHaveValue(kind.toLowerCase());
    }
    await expect(editor.getByText('Selected key is unavailable for this kind. Unsaved selection retained.', { exact: true })).toHaveCount(4);
    await editor.getByRole('combobox', { name: 'Backup root', exact: true }).selectOption('');
    await expect(editor.getByText('No optional root selected.', { exact: true })).toBeVisible();
  });

  test('loads every catalog page before publishing choices and rejects a changed generation', async ({ app, page }) => {
    const first = catalog();
    first.generation.slot_count = 2;
    const key = Buffer.from('library');
    const length = Buffer.alloc(2);
    length.writeUInt16BE(key.length);
    const next = Buffer.concat([length, key, Buffer.from('00000000000000000000000000000044', 'hex')]).toString('base64url');
    const second = catalog();
    second.generation.slot_count = 2;
    second.slots[0] = { ...second.slots[0],
      media_root_catalog_slot_public_id: '00000000-0000-0000-0000-000000000045',
      logical_key: 'z-library', requested_path: `${requestedPath}-two`, canonical_path: `${canonicalPath}-two`,
      filesystem_inode: '0000000000000003', root_identity_sha256: 'd'.repeat(64),
    };
    let pages = 0;
    await page.route(catalogRoute, async route => {
      pages += 1;
      const cursor = new URL(route.request().url()).searchParams.get('cursor');
      if (cursor) {
        expect(cursor).toBe(next);
        await route.fulfill({ json: second });
      } else {
        await route.fulfill({ json: { ...first, next_cursor: next } });
      }
    });
    await app.goto('/media');
    const editor = page.getByTestId('media-association-editor');
    await expect(editor.getByRole('combobox', { name: 'Source root', exact: true }).locator('option')).toHaveCount(3);
    expect(pages).toBe(2);
    second.generation.generation = '2';
    await page.getByRole('button', { name: 'Reload root catalog' }).click();
    await expect(page.getByTestId('media-root-catalog').getByRole('alert')).toHaveText('The catalog response is inconsistent. Reload before configuring associations.');
    await expect(editor.getByRole('combobox', { name: 'Source root', exact: true }).locator('option')).toHaveCount(1);
    await expect(editor.getByRole('button', { name: 'Create association', exact: true })).toBeDisabled();
  });
});
