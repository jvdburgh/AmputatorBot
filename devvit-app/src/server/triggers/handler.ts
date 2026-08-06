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
import { extractUrls } from '../core/urlExtract.ts';
import type { InstallSettings } from '../settings.ts';
import { type DedupRedis, isHandled, markHandled } from '../storage/dedup.ts';

// Just the two write methods off the real `RedditClient` — keeps the
// handler decoupled from the full client surface and trivial to stub in
// tests, while still picking up signature drift from Devvit upstream.
// `sendPrivateMessage` is only used by the mention flow's error fallback.
export type ReplyReddit = Pick<RedditClient, 'submitComment' | 'sendPrivateMessage'>;

// `comment` → comment-submit, parent is the comment itself (`t1_<id>`).
// `post` → post-submit, parent is the post (`t3_<id>`); reply is posted as
// a top-level comment on the post.
// `mention` → mention-in-comment (summon): mirrors the legacy bot — `id` and
// `body` are the mentioning comment's PARENT (the AMP-carrying comment or
// post), so the reply lands under the link it corrects, and `summoner`
// carries who tagged the bot. The summoner is notified via the u/-mention
// in the reply's credit line, or by DM when the reply can't be posted. See
// `triggers/mention.ts` for the parent fetch.
export type TriggerType = 'comment' | 'post' | 'mention';

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
  // The author of whatever `body` came from: the comment/post submitter for
  // the auto-reply flows, the PARENT's author for mentions. Used for the
  // self-reply guard. `undefined` when unavailable (system / deleted users).
  author: string | undefined;
  // Mention flow only: who tagged the bot. Drives the reply's
  // "Summoned by u/..." credit line (whose u/-mention is what notifies the
  // summoner) and the error-DM fallback. Leave unset for the other flows.
  summoner?: string;
  // Mention flow only: site-relative permalink of the summoning comment
  // (`event.comment.permalink`). Linked from the credit line so readers of
  // the reply can see who asked and where.
  summonPermalink?: string;
};

export type TriggerOutcome =
  | { status: 'replied' }
  // Mention flow only: the reply couldn't be posted (locked thread, removed
  // parent, bot banned), so the summoner got the canonicals by DM instead.
  | { status: 'dm_fallback'; reason: string }
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
  // usernames are case-insensitive, so compare case-folded. The summoner is
  // checked too: every reply footer now advertises "Summon: u/AmputatorBot",
  // and while Reddit doesn't fire mention events for self-authored mentions,
  // this makes a feedback loop impossible rather than just improbable.
  const isBot = (name: string | undefined) =>
    deps.botUsername.length > 0 &&
    name !== undefined &&
    name.toLowerCase() === deps.botUsername.toLowerCase();
  if (isBot(input.author) || isBot(input.summoner)) {
    return { status: 'skipped', reason: 'bot_self_reply' };
  }

  if (!deps.settings.autoReply) {
    return { status: 'skipped', reason: 'auto_reply_off' };
  }

  // Dedup scope follows the target's fullname, not the trigger kind, so a
  // summon on a comment the auto-reply flow already answered (or vice versa)
  // short-circuits here instead of double-posting the same canonicals.
  const scope = input.id.startsWith('t3_') ? 'post' : 'comment';
  if (await isHandled(deps.redis, scope, input.id)) {
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
    await markHandled(deps.redis, scope, input.id);
    return { status: 'skipped', reason: 'no_amp_urls' };
  }

  // Send only the AMP URLs the local check flagged. The backend re-extracts
  // and re-checks anyway, but a focused query saves it work.
  const query = ampUrls.join(' ');
  const entryType =
    input.kind === 'comment' ? 'COMMENT' : input.kind === 'post' ? 'SUBMISSION' : 'MENTION';

  const result = await deps.backend.convert({
    query,
    entryType,
    customFooter: deps.settings.customFooter,
  });
  if (!result.ok) {
    if (result.kind === 'no_amp') {
      // Local heuristic flagged the URL but the backend's stricter resolver
      // disagreed. Mark handled so we don't keep re-asking on retries.
      await markHandled(deps.redis, scope, input.id);
      return { status: 'skipped', reason: 'backend_no_amp' };
    }
    // Real failure (network, server error, invalid input). Do NOT mark
    // handled — let a retry try again once the upstream is healthy.
    return { status: 'error', reason: `${result.kind}: ${result.message}` };
  }

  if (result.comment === null) {
    // Backend resolved everything but found no canonical worth replying
    // about (e.g. all candidates were themselves AMP with no fallback).
    // Treat as handled — re-resolving won't help.
    await markHandled(deps.redis, scope, input.id);
    return { status: 'skipped', reason: 'no_canonical_to_share' };
  }

  // Summons credit the summoner under the backend-generated comment. The
  // u/-mention doubles as their notification: the reply goes to the parent,
  // so Reddit's reply notification reaches the parent's author, not them.
  // The u/-mention stays plain text (mentions inside link syntax don't
  // notify); "Summoned" carries the link back to the summoning comment.
  const text =
    input.kind === 'mention' && input.summoner
      ? `${result.comment}\n\n${summonCredit(input.summoner, input.summonPermalink)}`
      : result.comment;

  // `submitComment` accepts both t1_ and t3_ fullnames on `id` — see
  // `node_modules/.../@devvit/reddit/RedditClient.d.ts#submitComment`.
  try {
    await deps.reddit.submitComment({ id: input.id, text });
  } catch (err) {
    // Legacy parity: when a summoned reply can't be posted (locked thread,
    // removed parent, bot banned), the summoner still gets the answer — by
    // DM. Auto-reply flows have no one waiting, so for them the error
    // propagates and the trigger retry takes another swing.
    if (input.kind !== 'mention' || !input.summoner) throw err;
    await deps.reddit.sendPrivateMessage({
      to: input.summoner,
      subject: "AmputatorBot couldn't reply to your summon",
      text: `You summoned AmputatorBot, but posting a reply failed — the thread may be locked or the parent removed. Here's what it would have said:\n\n---\n\n${result.comment}`,
    });
    await markHandled(deps.redis, scope, input.id);
    return { status: 'dm_fallback', reason: String(err) };
  }
  await markHandled(deps.redis, scope, input.id);
  return { status: 'replied' };
}

// "[Summoned](url) by u/xyz" — linkless when the trigger payload carried no
// permalink. Devvit sends permalinks as site-relative paths.
function summonCredit(summoner: string, permalink: string | undefined): string {
  if (!permalink) return `Summoned by u/${summoner}`;
  const url = permalink.startsWith('/') ? `https://www.reddit.com${permalink}` : permalink;
  return `[Summoned](${url}) by u/${summoner}`;
}
