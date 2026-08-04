// Parent lookup for the mention trigger (`onMentionInCommentCreate`) — the
// summon feature, restored from the legacy bot (see the praw archive's
// `check_inbox.py`): a user replies to a comment or post that contains an
// AMP link and tags the bot; the bot resolves the links in that PARENT and
// replies under it, crediting the summoner with a u/-mention (which is what
// notifies them — see handler.ts).
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

export type MentionParent = {
  // The parent's fullname — reply target and dedup key for the summon.
  id: T1 | T3;
  // The parent's text, ready for URL extraction.
  text: string;
  // The parent's author, for the handler's self-reply guard (summoning the
  // bot on its own reply must not make it answer itself).
  authorName: string | undefined;
};

// Returns null when the parent fullname has an unexpected prefix (nothing
// to resolve — end the summon quietly). Fetch failures (deleted parent,
// permissions) propagate to the caller, which logs and drops the event.
export async function fetchMentionParent(
  reddit: MentionReddit,
  parentId: string,
): Promise<MentionParent | null> {
  if (parentId.startsWith('t1_')) {
    const parent = await reddit.getCommentById(T1(parentId));
    return { id: T1(parentId), text: parent.body, authorName: parent.authorName };
  }
  if (parentId.startsWith('t3_')) {
    // Same title + url + selftext join as the post-submit trigger — link
    // posts carry the AMP URL in `url`, self-posts in the body.
    const post = await reddit.getPostById(T3(parentId));
    return {
      id: T3(parentId),
      text: [post.title, post.url, post.body].filter(Boolean).join('\n'),
      authorName: post.authorName,
    };
  }
  return null;
}
