// Parent lookup for the mention trigger (`onMentionInCommentCreate`) — the
// summon feature, restored from the legacy bot (see the praw archive's
// `check_inbox.py`): a user replies to a comment or post that contains an
// AMP link and tags the bot; the bot resolves the links in that PARENT and
// answers the summoner.
//
// Deliberately parent-only: AMP links inside the mentioning comment itself
// already get handled by the regular comment-submit trigger, so reading
// them here too would double-reply.

import type { RedditClient } from '@devvit/web/server';
import { T1, T3 } from '@devvit/web/shared';

// Same pattern as `ReplyReddit` in handler.ts — narrow the client to what
// the mention flow needs so tests stub two methods instead of the full
// RedditClient surface.
export type MentionReddit = Pick<RedditClient, 'getCommentById' | 'getPostById'>;

// Returns the parent's text for URL extraction, or null when the parent
// fullname has an unexpected prefix (nothing to resolve — end the summon
// quietly). Fetch failures (deleted parent, permissions) propagate to the
// caller, which logs and drops the event.
export async function fetchMentionParentText(
  reddit: MentionReddit,
  parentId: string,
): Promise<string | null> {
  if (parentId.startsWith('t1_')) {
    const parent = await reddit.getCommentById(T1(parentId));
    return parent.body;
  }
  if (parentId.startsWith('t3_')) {
    // Same title + url + selftext join as the post-submit trigger — link
    // posts carry the AMP URL in `url`, self-posts in the body.
    const post = await reddit.getPostById(T3(parentId));
    return [post.title, post.url, post.body].filter(Boolean).join('\n');
  }
  return null;
}
