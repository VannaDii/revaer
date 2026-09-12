import assert from 'node:assert/strict';
import net from 'node:net';
import path from 'node:path';
import test from 'node:test';

import { E2E_SERVING_ENTRY, e2eServingCommand, requirePortFree } from '../global-setup';

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
