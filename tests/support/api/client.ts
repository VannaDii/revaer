import createClient from 'openapi-fetch';
import type { paths } from './schema';
import { recordApiCoverage } from './coverage';

export type ApiClient = ReturnType<typeof createClient<paths>>;

type ApiClientOptions = {
  baseUrl: string;
  headers?: Record<string, string>;
};

export function createApiClient(options: ApiClientOptions): ApiClient {
  const client = createClient<paths>({
    baseUrl: options.baseUrl,
    headers: options.headers,
  });

  client.use({
    onRequest({ request, schemaPath }) {
      recordApiCoverage(request.method, schemaPath);
    },
  });
  return client;
}
