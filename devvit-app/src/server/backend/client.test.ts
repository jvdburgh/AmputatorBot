// Unit tests for BackendClient with `fetch` mocked. Verifies the
// status-code → discriminated-result mapping the trigger handler relies on,
// the envelope-aware response parsing, and that the entryType header +
// generateMarkdownComment body field are wired through.

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { BackendClient, ENTRY_TYPE_HEADER } from './client.ts';
import type { ConvertResponseV2 } from './types.ts';

const SAMPLE_ENVELOPE: ConvertResponseV2 = {
  links: [
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
  ],
  comment: 'It looks like you shared an AMP link. AMP is supposed to be faster, but it…',
};

describe('BackendClient', () => {
  let fetchSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    fetchSpy = vi.spyOn(globalThis, 'fetch');
  });
  afterEach(() => {
    fetchSpy.mockRestore();
  });

  it('returns ok with parsed links + comment on 200', async () => {
    fetchSpy.mockResolvedValue(
      new Response(JSON.stringify(SAMPLE_ENVELOPE), {
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
      expect(result.links).toEqual(SAMPLE_ENVELOPE.links);
      expect(result.comment).toEqual(SAMPLE_ENVELOPE.comment);
    }
  });

  it('sends entryType via header, generateMarkdownComment via body, POSTs to /api/v2/convert', async () => {
    fetchSpy.mockResolvedValue(new Response(JSON.stringify(SAMPLE_ENVELOPE), { status: 200 }));

    const client = new BackendClient({ baseUrl: 'https://example.test/' }); // trailing slash
    await client.convert({
      query: 'https://example.com',
      entryType: 'SUBMISSION',
      customFooter: '[Modmail us](https://reddit.com/r/Sub)',
    });

    expect(fetchSpy).toHaveBeenCalledOnce();
    const call = fetchSpy.mock.calls[0];
    if (!call) throw new Error('fetch was not called');
    const [url, init] = call;
    expect(url).toBe('https://example.test/api/v2/convert');
    expect(init?.method).toBe('POST');
    const headers = init?.headers as Record<string, string>;
    expect(headers[ENTRY_TYPE_HEADER]).toBe('SUBMISSION');
    expect(headers['Content-Type']).toBe('application/json');
    const body = JSON.parse(init?.body as string);
    expect(body).toEqual({
      query: 'https://example.com',
      generateMarkdownComment: true,
      customFooter: '[Modmail us](https://reddit.com/r/Sub)',
    });
    // entryType is never in the body — it lives in the header.
    expect(body).not.toHaveProperty('entryType');
  });

  it('omits customFooter from the body when the option is unset', async () => {
    fetchSpy.mockResolvedValue(new Response(JSON.stringify(SAMPLE_ENVELOPE), { status: 200 }));

    const client = new BackendClient({ baseUrl: 'https://example.test' });
    await client.convert({ query: 'https://example.com', entryType: 'COMMENT' });

    const body = JSON.parse(fetchSpy.mock.calls[0]?.[1]?.body as string);
    expect(body).not.toHaveProperty('customFooter');
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
