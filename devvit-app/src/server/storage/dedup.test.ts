// Dedup helper tests against an in-memory Redis stub. We don't test the
// real Devvit Redis here — that's covered by `devvit playtest r/test`.

import { describe, expect, it } from 'vitest';
import { DEDUP_TTL_SECONDS, type DedupRedis, isHandled, markHandled } from './dedup.ts';

type StubRedis = DedupRedis & { _store: Map<string, string>; _ttl: Map<string, number> };

function memRedis(): StubRedis {
  const store = new Map<string, string>();
  const ttl = new Map<string, number>();
  const r: StubRedis = {
    _store: store,
    _ttl: ttl,
    set(key: string, value: string): Promise<string> {
      store.set(key, value);
      return Promise.resolve('OK');
    },
    expire(key: string, seconds: number): Promise<void> {
      ttl.set(key, seconds);
      return Promise.resolve();
    },
    exists(...keys: string[]): Promise<number> {
      return Promise.resolve(keys.filter((k) => store.has(k)).length);
    },
  };
  return r;
}

describe('dedup', () => {
  it('isHandled returns false for a fresh id', async () => {
    const r = memRedis();
    expect(await isHandled(r, 'comment', 't1_abc')).toBe(false);
  });

  it('markHandled then isHandled returns true', async () => {
    const r = memRedis();
    await markHandled(r, 'comment', 't1_abc');
    expect(await isHandled(r, 'comment', 't1_abc')).toBe(true);
  });

  it('namespaces comment and post separately', async () => {
    const r = memRedis();
    await markHandled(r, 'comment', 'xyz');
    expect(await isHandled(r, 'comment', 'xyz')).toBe(true);
    expect(await isHandled(r, 'post', 'xyz')).toBe(false);
  });

  it('applies the default TTL on mark', async () => {
    const r = memRedis();
    await markHandled(r, 'post', 't3_def');
    expect(r._ttl.get('handled:post:t3_def')).toBe(DEDUP_TTL_SECONDS);
  });
});
