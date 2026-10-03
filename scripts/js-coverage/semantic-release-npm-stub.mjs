import stub from '../../vendor/semantic-release-npm-stub/index.js';

try {
  for (const hook of Object.values(stub)) {
    await hook();
  }
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
}
