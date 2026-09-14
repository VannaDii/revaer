import type { AuthMode } from '../session';

export type SetupSnapshot = {
  app_profile?: Record<string, unknown>;
  fs_policy?: Record<string, unknown>;
};

export function setupChangeset(
  snapshot: SetupSnapshot | undefined,
  authMode: AuthMode,
  fsRoot: string,
): Record<string, unknown> {
  const appProfile = snapshot?.app_profile;
  if (!appProfile) {
    throw new Error('Snapshot missing app_profile for setup changeset.');
  }
  const fsPolicy = snapshot?.fs_policy;
  if (!fsPolicy) {
    throw new Error('Snapshot missing fs_policy for setup changeset.');
  }
  return {
    app_profile: { ...appProfile, auth_mode: authMode },
    fs_policy: { ...fsPolicy, allow_paths: [fsRoot] },
  };
}
