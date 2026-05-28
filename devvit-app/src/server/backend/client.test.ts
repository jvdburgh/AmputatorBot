// Unit tests for BackendClient with `fetch` mocked. Verifies the status-code
// → discriminated-result mapping the trigger handler relies on.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { BackendClient } from './client.ts';
import type { Link } from './types.ts';

const SAMPLE_LINKS: Link[] = [
  {
    origin: {
      domain: 'google',
      url: 'https://www.google.com/amp/s/example.eu/article',
      isAmp: true,
      isCached: true,
      isValid: true,
    },
    canonical: {
      domain: 'example',
      url: 'https://example.eu/article',
      type: 'REL',
      isAmp: false,
      isCached: false,
      isValid: true,
      isAlt: false,
      urlSimilarity: null,
    },
    canonicals: [],
    ampCanonical: null,
  },
];

describe('BackendClient', () => {
  let fetchSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    fetchSpy = vi.spyOn(globalThis, 'fetch');
  });
  afterEach(() => {
    fetchSpy.mockRestore();
  });

  it('returns ok with parsed links on 200', async () => {
    fetchSpy.mockResolvedValue(
      new Response(JSON.stringify(SAMPLE_LINKS), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      }),
    );

    const client = new BackendClient({ baseUrl: 'https://example.test' });
    const result = await client.convert({
      query: 'https://www.google.com/amp/s/example.eu/article',
      entryType: 'COMMENT',
    });

    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.links).toEqual(SAMPLE_LINKS);
    }
  });

  it('sends entryType, query, and POSTs to /api/v2/convert', async () => {
    fetchSpy.mockResolvedValue(new Response(JSON.stringify(SAMPLE_LINKS), { status: 200 }));

    const client = new BackendClient({ baseUrl: 'https://example.test/' }); // trailing slash
    await client.convert({
      query: 'https://example.com',
      entryType: 'SUBMISSION',
    });

    expect(fetchSpy).toHaveBeenCalledOnce();
    const call = fetchSpy.mock.calls[0];
    if (!call) throw new Error('fetch was not called');
    const [url, init] = call;
    expect(url).toBe('https://example.test/api/v2/convert');
    expect(init?.method).toBe('POST');
    const body = JSON.parse(init?.body as string);
    expect(body).toEqual({
      query: 'https://example.com',
      entryType: 'SUBMISSION',
    });
  });

  it('classifies a 406 error_no_amp as kind: no_amp', async () => {
    fetchSpy.mockResolvedValue(
      new Response(
        JSON.stringify({
          errorMessage: 'no AMP detected',
          resultCode: 'error_no_amp',
        }),
        { status: 406 },
      ),
    );

    const client = new BackendClient({ baseUrl: 'https://example.test' });
    const result = await client.convert({ query: 'https://example.com', entryType: 'COMMENT' });

    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.kind).toBe('no_amp');
  });

  it('classifies a 400 as invalid_input', async () => {
    fetchSpy.mockResolvedValue(
      new Response(JSON.stringify({ errorMessage: 'bad body' }), { status: 400 }),
    );

    const client = new BackendClient({ baseUrl: 'https://example.test' });
    const result = await client.convert({ query: '', entryType: 'COMMENT' });

    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.kind).toBe('invalid_input');
  });

  it('classifies a 500 as server_error', async () => {
    fetchSpy.mockResolvedValue(new Response('boom', { status: 500 }));

    const client = new BackendClient({ baseUrl: 'https://example.test' });
    const result = await client.convert({ query: 'https://x', entryType: 'COMMENT' });

    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.kind).toBe('server_error');
  });

  it('classifies a network failure as network_error', async () => {
    fetchSpy.mockRejectedValue(new Error('ECONNREFUSED'));

    const client = new BackendClient({ baseUrl: 'https://example.test' });
    const result = await client.convert({ query: 'https://x', entryType: 'COMMENT' });

    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.kind).toBe('network_error');
      expect(result.message).toContain('ECONNREFUSED');
    }
  });
});
