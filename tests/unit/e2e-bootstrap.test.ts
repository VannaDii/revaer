import assert from 'node:assert/strict';
import fs from 'node:fs';
import net from 'node:net';
import path from 'node:path';
import test from 'node:test';

import { E2E_SERVING_ENTRY, e2eServingCommand, requirePortFree } from '../global-setup';
import { setupChangeset } from '../support/api/setup-changeset';
import { repoRoot, resolveFsRoot } from '../support/paths';

const root = path.resolve('/fixture/current-worktree');
const executable = path.resolve('/fixture/target with spaces/debug/deps/revaer_app-012345');
const artifact = {
  reason: 'compiler-artifact',
  target: {
    name: 'revaer_app',
    kind: ['lib'],
    src_path: path.join(root, 'crates/revaer-app/src/lib.rs'),
  },
  profile: { test: true },
  executable,
};
const finished = { reason: 'build-finished', success: true };
const output = (...messages: unknown[]) => messages.map((value) => JSON.stringify(value)).join('\n');

test('selects only the current app library test executable and exact serving entry', () => {
  const command = e2eServingCommand(output(
    { reason: 'build-script-executed', package_id: 'unrelated' },
    { ...artifact, target: { ...artifact.target, name: 'dependency' } },
    { ...artifact, profile: { test: false }, executable: null },
    artifact,
    finished,
  ) + '\n', root);
  assert.deepEqual(command, {
    executable,
    args: ['--exact', 'bootstrap::runtime_tests::e2e_serving_entry', '--nocapture'],
  });
  assert.equal(E2E_SERVING_ENTRY, 'bootstrap::runtime_tests::e2e_serving_entry');
});

for (const [label, messages] of [
  ['missing finish', [artifact]],
  ['failed finish', [artifact, { ...finished, success: false }]],
  ['duplicate finish', [artifact, finished, finished]],
  ['missing artifact', [finished]],
  ['duplicate artifact', [artifact, artifact, finished]],
  ['production profile', [{ ...artifact, profile: { test: false } }, finished]],
  ['missing profile', [{ ...artifact, profile: null }, finished]],
  ['production binary', [{ ...artifact, target: { ...artifact.target, kind: ['bin'] } }, finished]],
  ['ambiguous target kind', [{ ...artifact, target: { ...artifact.target, kind: ['lib', 'bin'] } }, finished]],
  ['other worktree', [{ ...artifact, target: { ...artifact.target, src_path: '/other/src/lib.rs' } }, finished]],
  ['other crate', [{ ...artifact, target: { ...artifact.target, name: 'revaer_api' } }, finished]],
  ['relative executable', [{ ...artifact, executable: 'target/debug/revaer-app' }, finished]],
  ['missing executable', [{ ...artifact, executable: null }, finished]],
  ['wrong executable type', [{ ...artifact, executable: 42 }, finished]],
] as const) {
  test(`rejects ${label} instead of selecting a stale executable`, () => {
    assert.throws(() => e2eServingCommand(output(...messages), root));
  });
}

test('malformed Cargo JSON fails closed', () => {
  assert.throws(() => e2eServingCommand(`${output(artifact)}\nnot-json\n${output(finished)}`, root));
});

test('occupied ports are refused without terminating their owner', async () => {
  const server = net.createServer((socket) => socket.end());
  await new Promise<void>((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  const address = server.address();
  assert.ok(address && typeof address !== 'string');
  try {
    await assert.rejects(requirePortFree(address.port), /Port \d+ is in use/);
    assert.equal(server.listening, true);
  } finally {
    await new Promise<void>((resolve, reject) => server.close((error) => error ? reject(error) : resolve()));
  }
  await requirePortFree(address.port);
});

for (const authMode of ['none', 'api_key'] as const) {
  for (const rootKind of ['absolute', 'relative'] as const) {
    test(`setup allowlists the ${rootKind} torrent fixture root for ${authMode} auth`, () => {
      const scratch = fs.mkdtempSync(path.join(repoRoot(), 'target', 'e2e-authoring-'));
      const previousFsRoot = process.env.E2E_FS_ROOT;
      const fsPolicy = {
        id: 'filesystem-policy',
        library_root: '.server_root/library',
        allow_paths: ['.server_root/downloads', '.server_root/library'],
        move_mode: 'copy',
        cleanup_keep: ['*.txt'],
      };
      const appProfile = { id: 'app-profile', auth_mode: 'none', instance_name: 'E2E' };
      try {
        process.env.E2E_FS_ROOT = rootKind === 'absolute'
          ? scratch
          : path.relative(repoRoot(), scratch);
        assert.deepEqual(fs.readdirSync(scratch), []);
        const authorRoot = fs.mkdtempSync(path.join(resolveFsRoot(), 'e2e-author-'));
        fs.writeFileSync(path.join(authorRoot, 'seed.txt'), 'revaer e2e');

        const snapshot = { app_profile: appProfile, fs_policy: fsPolicy };
        const originalSnapshot = structuredClone(snapshot);
        const setupBody = setupChangeset(snapshot, authMode, resolveFsRoot());

        assert.deepEqual(setupBody, {
          app_profile: { ...appProfile, auth_mode: authMode },
          fs_policy: { ...fsPolicy, allow_paths: [scratch] },
        });
        assert.equal(path.dirname(authorRoot), scratch);
        assert.equal(fs.readFileSync(path.join(authorRoot, 'seed.txt'), 'utf-8'), 'revaer e2e');
        assert.deepEqual(snapshot, originalSnapshot);
      } finally {
        if (previousFsRoot === undefined) {
          delete process.env.E2E_FS_ROOT;
        } else {
          process.env.E2E_FS_ROOT = previousFsRoot;
        }
        fs.rmSync(scratch, { recursive: true, force: true });
      }
      assert.equal(fs.existsSync(scratch), false);
    });
  }
}

for (const [label, snapshot, missingField] of [
  ['missing snapshot', undefined, 'app_profile'],
  ['missing app profile', { fs_policy: {} }, 'app_profile'],
  ['missing filesystem policy', { app_profile: {} }, 'fs_policy'],
] as const) {
  test(`setup rejects ${label} instead of inventing fixture configuration`, () => {
    assert.throws(
      () => setupChangeset(snapshot, 'none', root),
      new Error(`Snapshot missing ${missingField} for setup changeset.`),
    );
  });
}
