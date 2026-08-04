// The mention flow has two halves: `fetchMentionParentText` (parent lookup,
// tested here with a stubbed Reddit client) and the shared `handleAmpTrigger`
// orchestration (mention-specific behavior covered in handler.test.ts).

import { describe, expect, it, vi } from 'vitest';
import { fetchMentionParentText, type MentionReddit } from './mention.ts';

function stubReddit(overrides: Partial<MentionReddit> = {}): MentionReddit {
  return {
    getCommentById: vi.fn(() =>
      Promise.resolve({ body: 'parent comment with https://amp.example.eu/a' }),
    ),
    getPostById: vi.fn(() =>
      Promise.resolve({
        title: 'A title',
        url: 'https://amp.example.eu/post',
        body: 'selftext here',
      }),
    ),
    ...overrides,
  } as unknown as MentionReddit;
}

describe('fetchMentionParentText', () => {
  it('returns the parent comment body for a t1_ parent', async () => {
    const reddit = stubReddit();
    const text = await fetchMentionParentText(reddit, 't1_parent');

    expect(text).toBe('parent comment with https://amp.example.eu/a');
    expect(reddit.getCommentById).toHaveBeenCalledWith('t1_parent');
    expect(reddit.getPostById).not.toHaveBeenCalled();
  });

  it('joins title + url + selftext for a t3_ parent', async () => {
    const reddit = stubReddit();
    const text = await fetchMentionParentText(reddit, 't3_parent');

    expect(text).toBe('A title\nhttps://amp.example.eu/post\nselftext here');
    expect(reddit.getPostById).toHaveBeenCalledWith('t3_parent');
    expect(reddit.getCommentById).not.toHaveBeenCalled();
  });

  it('drops empty selftext from the t3_ join', async () => {
    const reddit = stubReddit({
      getPostById: vi.fn(() =>
        Promise.resolve({ title: 'A title', url: 'https://amp.example.eu/post', body: undefined }),
      ) as unknown as MentionReddit['getPostById'],
    });
    const text = await fetchMentionParentText(reddit, 't3_parent');
    expect(text).toBe('A title\nhttps://amp.example.eu/post');
  });

  it('returns null for an unexpected fullname prefix', async () => {
    const text = await fetchMentionParentText(stubReddit(), 't4_message');
    expect(text).toBeNull();
  });

  it('propagates fetch failures to the caller', async () => {
    const reddit = stubReddit({
      getCommentById: vi.fn(() =>
        Promise.reject(new Error('deleted')),
      ) as unknown as MentionReddit['getCommentById'],
    });
    await expect(fetchMentionParentText(reddit, 't1_gone')).rejects.toThrow('deleted');
  });
});
