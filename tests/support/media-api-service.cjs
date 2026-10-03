const { createMediaServiceRelay } = require('./media-service-relay.cjs');

async function serve() {
  const app = process.env.APP_CONTAINER;
  const relay = createMediaServiceRelay(app);
  try {
    await new Promise((resolve, reject) => {
      relay.server.once('error', reject);
      relay.server.listen(0, '127.0.0.1', resolve);
    });
    const address = relay.server.address();
    if (!address || typeof address === 'string') throw new Error('Missing media fixture TCP address');
    const baseUrl = 'http://127.0.0.1:' + address.port;
    const health = await fetch(baseUrl + '/health', { signal: AbortSignal.timeout(10000) });
    if (!health.ok) throw new Error('Owned media fixture health failed: ' + health.status);
    if ((await health.json()).mode !== 'setup') throw new Error('Owned media fixture must start in setup mode');
    const authMode = process.env.E2E_MEDIA_AUTH_MODE ?? 'api_key';
    if (!['none', 'api_key'].includes(authMode)) throw new Error('Unsupported media fixture authentication mode');
    async function request(method, route, body, headers = {}) {
      const response = await fetch(baseUrl + route, {
        method, signal: AbortSignal.timeout(10000),
        headers: { ...headers, ...(body === undefined ? {} : { 'Content-Type': 'application/json' }) },
        body: body === undefined ? undefined : JSON.stringify(body),
      });
      if (!response.ok) throw new Error('Media fixture bootstrap failed: ' + route + ' ' + response.status);
      return response.json();
    }
    const start = await request('POST', '/admin/setup/start', {});
    if (!start.token) throw new Error('Media fixture setup token is missing');
    const snapshot = await request('GET', '/.well-known/revaer.json');
    const setup = await request('POST', '/admin/setup/complete', {
      app_profile: { ...snapshot.app_profile, auth_mode: authMode },
      fs_policy: { ...snapshot.fs_policy, allow_paths: ['/proof'] },
    }, { 'x-revaer-setup-token': start.token });
    if (authMode === 'api_key' && !setup.api_key) throw new Error('Media fixture API key is missing');
    const database = new URL(process.env.DATABASE_URL).pathname.slice(1);
    if (!/^revaer_test_[0-9]+_[0-9]+$/.test(database)) throw new Error('Invalid owned fixture database');
    process.stdout.write('MEDIA_API_FIXTURE_READY ' + JSON.stringify({
      baseUrl, container: app, database, session: { authMode, apiKey: setup.api_key },
    }) + '\n');
    await new Promise((resolve, reject) => {
      process.stdin.once('end', resolve);
      process.stdin.once('error', reject);
      process.stdin.resume();
    });
  } finally {
    await relay.close();
  }
}

serve().catch(error => { console.error(error); process.exitCode = 1; });
