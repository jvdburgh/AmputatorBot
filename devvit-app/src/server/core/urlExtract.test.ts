// Mirrors `backend/src/canonical/url_extract.rs#tests`.

import { describe, expect, it } from 'vitest';
import { extractUrls, removeMarkdown } from './urlExtract.ts';

describe('extractUrls', () => {
  it('extracts a single URL', () => {
    expect(extractUrls('check this out: https://example.eu/article')).toEqual([
      'https://example.eu/article',
    ]);
  });

  it('deduplicates repeated URLs', () => {
    expect(extractUrls('https://example.eu and https://example.eu again')).toEqual([
      'https://example.eu',
    ]);
  });

  it('preserves source order across multiple URLs', () => {
    const body = 'first https://a.example then https://b.example, finally https://c.example';
    expect(extractUrls(body)).toEqual([
      'https://a.example',
      'https://b.example',
      'https://c.example',
    ]);
  });

  it('extracts from Reddit link markdown', () => {
    const body = 'see [the article](https://example.eu/news), or directly: https://example.eu/raw';
    const urls = extractUrls(body);
    expect(urls).toContain('https://example.eu/news');
    expect(urls).toContain('https://example.eu/raw');
  });

  it('handles text without URLs', () => {
    expect(extractUrls('no urls here just text')).toEqual([]);
  });

  it('handles empty input', () => {
    expect(extractUrls('')).toEqual([]);
  });

  it('extracts an unencoded Google AMP URL pasted as-is', () => {
    const body = 'https://www.google.com/amp/s/electrek.co/2018/06/19/tesla-model-3/amp/';
    const urls = extractUrls(body);
    expect(urls).toHaveLength(1);
    expect(urls[0]).toContain('/amp/s/electrek.co');
  });
});

describe('removeMarkdown', () => {
  it('strips trailing markdown punctuation', () => {
    expect(removeMarkdown('https://example.eu.')).toBe('https://example.eu');
    expect(removeMarkdown('https://example.eu,')).toBe('https://example.eu');
    expect(removeMarkdown('https://example.eu)')).toBe('https://example.eu');
    expect(removeMarkdown('https://example.eu?)')).toBe('https://example.eu');
    expect(removeMarkdown('https://example.eu”')).toBe('https://example.eu');
  });

  it('preserves a URL without trailing punctuation', () => {
    expect(removeMarkdown('https://example.eu')).toBe('https://example.eu');
    expect(removeMarkdown('https://example.eu/article')).toBe('https://example.eu/article');
  });
});
