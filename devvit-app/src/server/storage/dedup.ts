// Per-trigger dedup against Devvit Redis.
//
// Devvit retries triggers under some conditions (rolling deploys, transient
// errors), and `devvit playtest` reloads the server while leaving event
// state intact. Without dedup the bot would happily reply twice to the same
// comment. We mark each handled comment/post for 1 hour — long enough to
// absorb the retry windows we care about, short enough that the Redis usage
// stays trivial.
//
// Devvit's Redis API is a small subset of real Redis (see
// https://developers.reddit.com/docs/capabilities/server/redis). Notably
// `set` doesn't accept the `EX` option in one call — TTL has to be applied
// via a separate `expire`. We accept the two-call atomicity gap because the
// worst case is a dedup key with no TTL, which the next mark-handled call
// would reset.

import type { RedisClient } from '@devvit/web/server';

// Pick the minimum surface we need from Devvit's RedisClient. Lets tests
// stub a small object without satisfying the full ~50-method interface.
export type DedupRedis = Pick<RedisClient, 'set' | 'expire' | 'exists'>;

export type DedupScope = 'comment' | 'post';

// Default 1-hour TTL on every mark. Adjust here, not at call sites.
export const DEDUP_TTL_SECONDS = 60 * 60;

function dedupKey(scope: DedupScope, id: string): string {
  return `handled:${scope}:${id}`;
}

export async function isHandled(
  redis: DedupRedis,
  scope: DedupScope,
  id: string,
): Promise<boolean> {
  const count = await redis.exists(dedupKey(scope, id));
  return count > 0;
}

export async function markHandled(redis: DedupRedis, scope: DedupScope, id: string): Promise<void> {
  const key = dedupKey(scope, id);
  await redis.set(key, '1');
  await redis.expire(key, DEDUP_TTL_SECONDS);
}
