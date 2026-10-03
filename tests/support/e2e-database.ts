const LOCAL_HOSTS = new Set(['localhost', '127.0.0.1', 'host.docker.internal']);

export type TestDatabase = {
  name: string;
  runtimeUrl: string;
};

export function verifyTestDatabaseEndpoint(adminUrl: string, binding: string): void {
  const url = parseDatabaseUrl(adminUrl);
  const expected = `127.0.0.1:${url.port || '5432'}`;
  if (!LOCAL_HOSTS.has(url.hostname) || binding.trim() !== expected) {
    throw new Error('Test database URL does not match the selected container loopback port.');
  }
}

export function testDatabase(adminUrl: string, name: string, password: string): TestDatabase {
  if (!/^revaer_test_[0-9]+_[0-9]+$/.test(name) || name.length > 50) {
    throw new Error('Invalid owned test database name.');
  }
  if (!/^[0-9a-f]{64}$/.test(password)) {
    throw new Error('Invalid temporary runtime password.');
  }
  const url = parseDatabaseUrl(adminUrl);
  url.pathname = `/${name}`;
  url.username = `${name}_runtime`;
  url.password = password;
  return { name, runtimeUrl: url.toString() };
}

function parseDatabaseUrl(adminUrl: string): URL {
  let url: URL;
  try {
    url = new URL(adminUrl);
  } catch {
    throw new Error('Invalid test database service URL.');
  }
  if (!['postgres:', 'postgresql:'].includes(url.protocol) ||
      !LOCAL_HOSTS.has(url.hostname) || url.search || url.hash) {
    throw new Error('Test database requires an explicit local PostgreSQL URL without overrides.');
  }
  return url;
}
