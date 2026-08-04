// The mention flow has two halves: `fetchMentionParent` (parent lookup,
// tested here with a stubbed Reddit client) and the shared `handleAmpTrigger`
// orchestration (mention-specific behavior covered in handler.test.ts).

import { describe, expect, it, vi } from 'vitest';
import { fetchMentionParent, type MentionReddit } from './mention.ts';

function stubReddit(overrides: Partial<MentionReddit> = {}): MentionReddit {
  return {
    getCommentById: vi.fn(() =>
      Promise.resolve({
        body: 'parent comment with https://amp.example.eu/a',
        authorName: 'parent-author',
      }),
    ),
    getPostById: vi.fn(() =>
      Promise.resolve({
        title: 'A title',
        url: 'https://amp.example.eu/post',
        body: 'selftext here',
        authorName: 'post-author',
      }),
    ),
    ...overrides,
  } as unknown as MentionReddit;
}

describe('fetchMentionParent', () => {
  it('returns the parent comment body + author for a t1_ parent', async () => {
    const reddit = stubReddit();
    const parent = await fetchMentionParent(reddit, 't1_parent');

    expect(parent).toEqual({
      id: 't1_parent',
      text: 'parent comment with https://amp.example.eu/a',
      authorName: 'parent-author',
    });
    expect(reddit.getCommentById).toHaveBeenCalledWith('t1_parent');
    expect(reddit.getPostById).not.toHaveBeenCalled();
  });

  it('joins title + url + selftext for a t3_ parent', async () => {
    const reddit = stubReddit();
    const parent = await fetchMentionParent(reddit, 't3_parent');

    expect(parent).toEqual({
      id: 't3_parent',
      text: 'A title\nhttps://amp.example.eu/post\nselftext here',
      authorName: 'post-author',
    });
    expect(reddit.getPostById).toHaveBeenCalledWith('t3_parent');
    expect(reddit.getCommentById).not.toHaveBeenCalled();
  });

  it('drops empty selftext from the t3_ join', async () => {
    const reddit = stubReddit({
      getPostById: vi.fn(() =>
        Promise.resolve({
          title: 'A title',
          url: 'https://amp.example.eu/post',
          body: undefined,
          authorName: 'post-author',
        }),
      ) as unknown as MentionReddit['getPostById'],
    });
    const parent = await fetchMentionParent(reddit, 't3_parent');
    expect(parent?.text).toBe('A title\nhttps://amp.example.eu/post');
  });

  it('returns null for an unexpected fullname prefix', async () => {
    const parent = await fetchMentionParent(stubReddit(), 't4_message');
    expect(parent).toBeNull();
  });

  it('propagates fetch failures to the caller', async () => {
    const reddit = stubReddit({
      getCommentById: vi.fn(() =>
        Promise.reject(new Error('deleted')),
      ) as unknown as MentionReddit['getCommentById'],
    });
    await expect(fetchMentionParent(reddit, 't1_gone')).rejects.toThrow('deleted');
  });
});
