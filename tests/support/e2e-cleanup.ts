import { execFileSync } from 'child_process';

import { clearState, readState } from './e2e-state';
import { repoRoot } from './paths';

export async function cleanupE2EState(): Promise<void> {
  const state = readState();
  if (!state) {
    return;
  }

  await terminateProcess(state.uiPid);
  await terminateProcess(state.apiPid);
  if (state.dbUrl) {
    if (!state.testDatabaseName || !state.testDatabaseContainer) {
      throw new Error('Cannot clean up the test database without ownership metadata.');
    }
    execFileSync('just', ['db-test-drop', state.testDatabaseName], {
      cwd: repoRoot(),
      env: { ...process.env, PG_CONTAINER: state.testDatabaseContainer },
      stdio: 'inherit',
    });
  }
  clearState();
}

async function terminateProcess(pid?: number): Promise<void> {
  if (!pid || !isAlive(pid)) {
    return;
  }
  try {
    process.kill(-pid, 'SIGTERM');
  } catch {
    try {
      process.kill(pid, 'SIGTERM');
    } catch {
      return;
    }
  }
  for (let attempt = 0; attempt < 20; attempt += 1) {
    if (!isAlive(pid)) {
      return;
    }
    await delay(250);
  }
  try {
    process.kill(-pid, 'SIGKILL');
  } catch {
    try {
      process.kill(pid, 'SIGKILL');
    } catch {
      // ignore
    }
  }
}

function isAlive(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
