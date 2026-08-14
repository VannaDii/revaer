export type CoverageKind = 'API' | 'UI';

export function requireCoverage(kind: CoverageKind): boolean {
  const envName = `E2E_COVERAGE_REQUIRE_${kind}`;
  const value = process.env[envName];
  if (!value) {
    return true;
  }
  if (/^(1|true|TRUE|yes|YES|on|ON)$/.test(value)) {
    return true;
  }
  if (/^(0|false|FALSE|no|NO|off|OFF)$/.test(value)) {
    return false;
  }
  throw new Error(`${envName} must be a boolean value.`);
}
