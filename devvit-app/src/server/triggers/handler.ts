// Shared orchestration for the comment-submit and post-submit triggers.
//
// Kept side-effect-free (no top-level reads of the live Devvit clients) so
// it can be exercised end-to-end in Vitest by injecting stubs for `redis`,
// `reddit`, the backend client, settings, and the bot's username. The thin
// HTTP wiring in `index.ts` adapts the Hono request body + the live Devvit
// clients into a `TriggerInput` + `TriggerDeps`.

import type { RedditClient } from '@devvit/web/server';
import type { T1, T3 } from '@devvit/web/shared';

import type { BackendClient } from '../backend/client.ts';
import { isAmpUrl } from '../core/ampDetect.ts';
import { buildReply, type TriggerType } from '../core/reply.ts';
import { extractUrls } from '../core/urlExtract.ts';
import type { InstallSettings } from '../settings.ts';
import { type DedupRedis, isHandled, markHandled } from '../storage/dedup.ts';

// Just the `submitComment` method off the real `RedditClient` — keeps the
// handler decoupled from the full client surface and trivial to stub in
// tests, while still picking up signature drift from Devvit upstream.
export type ReplyReddit = Pick<RedditClient, 'submitComment'>;

export type TriggerDeps = {
  redis: DedupRedis;
  reddit: ReplyReddit;
  backend: BackendClient;
  settings: InstallSettings;
  // The Reddit username of the per-install app account, resolved once from
  // `reddit.getAppUser()` at server boot. Used to skip self-replies before
  // we touch any other state. Pass empty string to disable the guard (the
  // local AMP filter + dedup would still catch most loops, but the explicit
  // check is the load-bearing one — defense in depth on something that
  // would be very noisy if it went wrong).
  botUsername: string;
};

export type TriggerInput = {
  // 'comment' → comment-submit, parent is the comment itself (`t1_<id>`).
  // 'post' → post-submit, parent is the post (`t3_<id>`); reply is posted as
  // a top-level comment on the post.
  kind: TriggerType;
  // Fullname (`t1_...` for comments, `t3_...` for posts). The Devvit
  // `submitComment` call accepts both prefixes — comment replies to a comment,
  // post id replies as a top-level comment. Typed as the discriminated
  // template-literal union from `@devvit/web/shared` so we don't have to
  // cast at the call site.
  id: T1 | T3;
  // For comments: the comment body. For posts: title + (link URL) + selftext
  // joined with whitespace so all three are URL-extracted in one pass.
  body: string;
  // The username of whoever submitted the comment/post (`event.author?.name`).
  // Used for the self-reply guard. `undefined` when Devvit didn't include an
  // author (rare; system / deleted users).
  author: string | undefined;
};

export type TriggerOutcome =
  | { status: 'replied' }
  | { status: 'skipped'; reason: SkipReason }
  | { status: 'error'; reason: string };

export type SkipReason =
  | 'bot_self_reply'
  | 'auto_reply_off'
  | 'already_handled'
  | 'no_urls'
  | 'no_amp_urls'
  | 'backend_no_amp'
  | 'no_canonical_to_share';

export async function handleAmpTrigger(
  input: TriggerInput,
  deps: TriggerDeps,
): Promise<TriggerOutcome> {
  // Self-reply guard — must run before anything that touches state. Reddit
  // usernames are case-insensitive, so compare case-folded.
  if (
    deps.botUsername.length > 0 &&
    input.author &&
    input.author.toLowerCase() === deps.botUsername.toLowerCase()
  ) {
    return { status: 'skipped', reason: 'bot_self_reply' };
  }

  if (!deps.settings.autoReply) {
    return { status: 'skipped', reason: 'auto_reply_off' };
  }

  if (await isHandled(deps.redis, input.kind, input.id)) {
    return { status: 'skipped', reason: 'already_handled' };
  }

  const urls = extractUrls(input.body);
  if (urls.length === 0) {
    return { status: 'skipped', reason: 'no_urls' };
  }

  const ampUrls = urls.filter(isAmpUrl);
  if (ampUrls.length === 0) {
    // Almost all comments hit this path. Mark handled so a re-fire of the
    // same trigger doesn't re-run the extraction; cheap and bounds Redis
    // growth to the same 1h window we'd hit on real replies.
    await markHandled(deps.redis, input.kind, input.id);
    return { status: 'skipped', reason: 'no_amp_urls' };
  }

  // Send only the AMP URLs the local check flagged. The backend re-extracts
  // and re-checks anyway, but a focused query saves it work.
  const query = ampUrls.join(' ');
  const entryType = input.kind === 'comment' ? 'COMMENT' : 'SUBMISSION';

  const result = await deps.backend.convert({ query, entryType });
  if (!result.ok) {
    if (result.kind === 'no_amp') {
      // Local heuristic flagged the URL but the backend's stricter resolver
      // disagreed. Mark handled so we don't keep re-asking on retries.
      await markHandled(deps.redis, input.kind, input.id);
      return { status: 'skipped', reason: 'backend_no_amp' };
    }
    // Real failure (network, server error, invalid input). Do NOT mark
    // handled — let a retry try again once the upstream is healthy.
    return { status: 'error', reason: `${result.kind}: ${result.message}` };
  }

  const replyText = buildReply(result.links, {
    triggerType: input.kind,
    customFooter: deps.settings.customFooter,
  });
  if (replyText === null) {
    // Backend resolved everything but found no canonical worth replying
    // about (e.g. all candidates were themselves AMP with no fallback).
    // Treat as handled — re-resolving won't help.
    await markHandled(deps.redis, input.kind, input.id);
    return { status: 'skipped', reason: 'no_canonical_to_share' };
  }

  // `submitComment` accepts both t1_ and t3_ fullnames on `id` — see
  // `node_modules/.../@devvit/reddit/RedditClient.d.ts#submitComment`.
  await deps.reddit.submitComment({ id: input.id, text: replyText });
  await markHandled(deps.redis, input.kind, input.id);
  return { status: 'replied' };
}
