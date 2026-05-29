// Backend client — wraps `POST /api/v2/convert` on the AmputatorBot Rust
// backend. The bot asks the backend to generate the Reddit reply markdown
// for it (`generateMarkdownComment: true`) so reply rendering lives in
// exactly one place (Rust) instead of being duplicated client-side.
//
// `entryType` is sent via the `X-AmputatorBot-Entry-Type` HTTP header, not
// the body — the backend treats it as internal-only and won't accept it
// from random API callers, so per-source `links` analytics stay honest.
//
// No auth — the backend has no privileged routes (opt-out dropped). The
// bot only ever POSTs publicly accessible endpoints.

import type { ConvertResponseV2, EntryType } from './types.ts';

export type ConvertOptions = {
  query: string;
  // Sent via the X-AmputatorBot-Entry-Type header. `COMMENT` for comment
  // triggers, `SUBMISSION` for posts; the website would send `ONLINE`.
  entryType: EntryType;
  // Optional per-install mod-supplied footer addendum. Threaded into the
  // returned `comment` only when entryType is a bot variant; ignored by
  // the backend for `API` / `ONLINE`.
  customFooter?: string;
  // Defaults mirror the backend's serde defaults (true, 3). Override sparingly.
  guessAndCheck?: boolean;
  maxDepth?: number;
};

// Discriminated union — the trigger handler matches on `ok` and exits
// early on anything other than `ok: true`. `no_amp` is a legitimate
// outcome (the URL turned out not to be AMP after closer inspection) and
// gets logged at debug, not warn.
export type ConvertResult =
  | { ok: true; links: ConvertResponseV2['links']; comment: string | null }
  | {
      ok: false;
      kind: 'no_amp' | 'invalid_input' | 'server_error' | 'network_error';
      message: string;
    };

export type BackendClientConfig = {
  // Base URL without trailing slash, e.g. `https://www.amputatorbot.com`.
  // Pulled from the per-install setting at handler time.
  baseUrl: string;
  // Per-request timeout. Backend's canonical-finding can take a few seconds
  // when GUESS_AND_CHECK has to fetch + score, so this is intentionally
  // generous. The Devvit trigger has its own outer timeout.
  timeoutMs?: number;
};

export const ENTRY_TYPE_HEADER = 'X-AmputatorBot-Entry-Type';

export class BackendClient {
  readonly #baseUrl: string;
  readonly #timeoutMs: number;

  constructor(config: BackendClientConfig) {
    this.#baseUrl = config.baseUrl.replace(/\/$/, '');
    this.#timeoutMs = config.timeoutMs ?? 15_000;
  }

  async convert(options: ConvertOptions): Promise<ConvertResult> {
    const body = {
      query: options.query,
      generateMarkdownComment: true,
      ...(options.customFooter !== undefined && { customFooter: options.customFooter }),
      ...(options.guessAndCheck !== undefined && { guessAndCheck: options.guessAndCheck }),
      ...(options.maxDepth !== undefined && { maxDepth: options.maxDepth }),
    };

    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), this.#timeoutMs);

    let response: Response;
    try {
      response = await fetch(`${this.#baseUrl}/api/v2/convert`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          [ENTRY_TYPE_HEADER]: options.entryType,
        },
        body: JSON.stringify(body),
        signal: controller.signal,
      });
    } catch (err) {
      return {
        ok: false,
        kind: 'network_error',
        message: err instanceof Error ? err.message : String(err),
      };
    } finally {
      clearTimeout(timeout);
    }

    if (response.status === 200) {
      const envelope = (await response.json()) as ConvertResponseV2;
      return { ok: true, links: envelope.links, comment: envelope.comment };
    }

    // Backend's error responses: { errorMessage, resultCode }. 406 with
    // `error_no_amp` is the most common non-success path — the bot saw an
    // AMP-looking URL via the local heuristic, but the backend's stricter
    // resolver confirmed it's not actually AMP.
    let message = `HTTP ${response.status}`;
    let resultCode: string | undefined;
    try {
      const err = (await response.json()) as { errorMessage?: string; resultCode?: string };
      if (err.errorMessage) message = err.errorMessage;
      resultCode = err.resultCode;
    } catch {
      // Backend returned a non-JSON error body — keep the HTTP-status message.
    }

    if (response.status === 406 || resultCode === 'error_no_amp') {
      return { ok: false, kind: 'no_amp', message };
    }
    if (response.status === 400 || response.status === 422) {
      return { ok: false, kind: 'invalid_input', message };
    }
    return { ok: false, kind: 'server_error', message };
  }
}
