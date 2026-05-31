// End-to-end-ish tests for the trigger orchestration: every dep is stubbed,
// and we assert the outcome + the side effects (reddit reply + dedup mark).

import { describe, expect, it, vi } from 'vitest';
import type { BackendClient, ConvertResult } from '../backend/client.ts';
import type { Link } from '../backend/types.ts';
import type { InstallSettings } from '../settings.ts';
import type { DedupRedis } from '../storage/dedup.ts';
import {
  handleAmpTrigger,
  type ReplyReddit,
  type TriggerDeps,
  type TriggerInput,
} from './handler.ts';

const BOT_USERNAME = 'amputatorbot-r-test';

function memRedis(): DedupRedis {
  const store = new Map<string, string>();
  return {
    set(key: string, value: string): Promise<string> {
      store.set(key, value);
      return Promise.resolve('OK');
    },
    expire(_key: string, _seconds: number): Promise<void> {
      return Promise.resolve();
    },
    exists(...keys: string[]): Promise<number> {
      return Promise.resolve(keys.filter((k) => store.has(k)).length);
    },
  };
}

function stubBackend(result: ConvertResult): BackendClient {
  return {
    convert: vi.fn(() => Promise.resolve(result)),
  } as unknown as BackendClient;
}

function stubReddit(): ReplyReddit & { submitComment: ReturnType<typeof vi.fn> } {
  return { submitComment: vi.fn(() => Promise.resolve()) } as unknown as ReplyReddit & {
    submitComment: ReturnType<typeof vi.fn>;
  };
}

const settings = (overrides: Partial<InstallSettings> = {}): InstallSettings => ({
  autoReply: true,
  customFooter: undefined,
  ...overrides,
});

const ampInput: TriggerInput = {
  kind: 'comment',
  id: 't1_abc' as const,
  body: 'check this https://www.google.com/amp/s/example.eu/article',
  author: 'somebody-else',
};

function okResult(overrides: { comment?: string | null } = {}): ConvertResult {
  const links: Link[] = [
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
        articleSimilarity: null,
        confidenceScore: null,
        confidenceLevel: null,
      },
      canonicals: [],
      ampCanonical: null,
    },
  ];
  const comment =
    overrides.comment !== undefined
      ? overrides.comment
      : 'It looks like you shared an AMP link. AMP is supposed to be faster, but it — especially cached pages like the one you shared — is controversial because of [concerns over privacy and the Open Web](https://www.reddit.com/r/AmputatorBot/comments/ehrq3z/why_did_i_build_amputatorbot).\n\nMaybe check out **the canonical page** instead: **[https://example.eu/article](https://example.eu/article)**';
  return { ok: true, links, comment };
}

function deps(overrides: Partial<TriggerDeps> = {}): TriggerDeps {
  return {
    redis: memRedis(),
    reddit: stubReddit(),
    backend: stubBackend(okResult()),
    settings: settings(),
    botUsername: BOT_USERNAME,
    ...overrides,
  };
}

describe('handleAmpTrigger', () => {
  it('replies with canonical when an AMP URL is found', async () => {
    const reddit = stubReddit();
    const outcome = await handleAmpTrigger(ampInput, deps({ reddit }));

    expect(outcome).toEqual({ status: 'replied' });
    expect(reddit.submitComment).toHaveBeenCalledOnce();
    const [call] = reddit.submitComment.mock.calls;
    expect(call?.[0].id).toBe('t1_abc');
    expect(call?.[0].text).toContain(
      '**[https://example.eu/article](https://example.eu/article)**',
    );
  });

  it('posts the backend-generated comment verbatim', async () => {
    // The handler no longer generates the reply markdown — the API does,
    // server-side. The handler is just glue: it forwards `result.comment`
    // straight to `submitComment`.
    const reddit = stubReddit();
    const backend = stubBackend(okResult({ comment: 'CUSTOM COMMENT BODY' }));
    await handleAmpTrigger(ampInput, deps({ reddit, backend }));

    const text = reddit.submitComment.mock.calls[0]?.[0].text as string;
    expect(text).toBe('CUSTOM COMMENT BODY');
  });

  it('skips with bot_self_reply when the trigger author matches the app account', async () => {
    const reddit = stubReddit();
    const outcome = await handleAmpTrigger({ ...ampInput, author: BOT_USERNAME }, deps({ reddit }));
    expect(outcome).toEqual({ status: 'skipped', reason: 'bot_self_reply' });
    expect(reddit.submitComment).not.toHaveBeenCalled();
  });

  it('matches bot self-reply case-insensitively', async () => {
    const reddit = stubReddit();
    const outcome = await handleAmpTrigger(
      { ...ampInput, author: BOT_USERNAME.toUpperCase() },
      deps({ reddit }),
    );
    expect(outcome).toEqual({ status: 'skipped', reason: 'bot_self_reply' });
  });

  it('does NOT trigger bot_self_reply when botUsername is empty (defense-in-depth fallback)', async () => {
    const reddit = stubReddit();
    const outcome = await handleAmpTrigger(
      { ...ampInput, author: '' },
      deps({ reddit, botUsername: '' }),
    );
    // Falls through to the normal flow — would only short-circuit on
    // dedup / AMP filter / backend. Here it replies because the URL is AMP.
    expect(outcome).toEqual({ status: 'replied' });
  });

  it('skips with auto_reply_off when the setting is disabled', async () => {
    const reddit = stubReddit();
    const outcome = await handleAmpTrigger(
      ampInput,
      deps({ reddit, settings: settings({ autoReply: false }) }),
    );
    expect(outcome).toEqual({ status: 'skipped', reason: 'auto_reply_off' });
    expect(reddit.submitComment).not.toHaveBeenCalled();
  });

  it('skips with already_handled on a re-fire of the same trigger', async () => {
    const reddit = stubReddit();
    const shared = deps({ reddit });

    await handleAmpTrigger(ampInput, shared);
    const second = await handleAmpTrigger(ampInput, shared);

    expect(second).toEqual({ status: 'skipped', reason: 'already_handled' });
    expect(reddit.submitComment).toHaveBeenCalledOnce();
  });

  it('skips with no_urls when the body has no URLs', async () => {
    const outcome = await handleAmpTrigger(
      { kind: 'comment', id: 't1_no' as const, body: 'plain text, nothing to see', author: 'u' },
      deps(),
    );
    expect(outcome).toEqual({ status: 'skipped', reason: 'no_urls' });
  });

  it('skips with no_amp_urls when URLs exist but none are AMP, and marks handled', async () => {
    const shared = deps();
    const outcome = await handleAmpTrigger(
      {
        kind: 'comment',
        id: 't1_clean' as const,
        body: 'just https://example.eu/clean here',
        author: 'u',
      },
      shared,
    );

    expect(outcome).toEqual({ status: 'skipped', reason: 'no_amp_urls' });
    expect(shared.backend.convert as ReturnType<typeof vi.fn>).not.toHaveBeenCalled();
    // Re-firing should now short-circuit on dedup.
    const again = await handleAmpTrigger(
      {
        kind: 'comment',
        id: 't1_clean' as const,
        body: 'just https://example.eu/clean here',
        author: 'u',
      },
      shared,
    );
    expect(again).toEqual({ status: 'skipped', reason: 'already_handled' });
  });

  it('skips with backend_no_amp + marks handled when the backend disagrees with the local heuristic', async () => {
    const shared = deps({
      backend: stubBackend({ ok: false, kind: 'no_amp', message: 'no AMP detected' }),
    });
    const outcome = await handleAmpTrigger(ampInput, shared);
    expect(outcome).toEqual({ status: 'skipped', reason: 'backend_no_amp' });

    const again = await handleAmpTrigger(ampInput, {
      ...shared,
      backend: stubBackend(okResult()),
    });
    expect(again).toEqual({ status: 'skipped', reason: 'already_handled' });
  });

  it('returns error WITHOUT marking handled on a transient backend failure', async () => {
    const shared = deps({
      backend: stubBackend({ ok: false, kind: 'network_error', message: 'ECONNREFUSED' }),
    });
    const outcome = await handleAmpTrigger(ampInput, shared);
    expect(outcome.status).toBe('error');

    // A retry should NOT see already_handled — it should reach the backend again.
    const retry = await handleAmpTrigger(ampInput, {
      ...shared,
      backend: stubBackend(okResult()),
    });
    expect(retry).toEqual({ status: 'replied' });
  });
});
