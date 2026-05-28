// Mirrors `backend/src/canonical/amp_detect.rs#tests`. If you add a case here,
// add the same case to the Rust file and vice versa — the two impls must agree.

import { describe, expect, it } from 'vitest';
import { isAmpUrl, isCachedAmp } from './ampDetect.ts';

describe('isAmpUrl', () => {
  it('detects Google AMP cache URLs', () => {
    expect(isAmpUrl('https://www.google.com/amp/s/electrek.co/2018/06/19/tesla-model-3/amp/')).toBe(
      true,
    );
  });

  it('detects publisher AMP paths', () => {
    expect(isAmpUrl('https://www.bbc.com/news/world-europe-12345/amp')).toBe(true);
    expect(isAmpUrl('https://www.fox5atlanta.com/video/foo.amp')).toBe(true);
  });

  it('detects amp subdomain', () => {
    expect(isAmpUrl('https://amp.cnn.com/cnn/2020/some-article')).toBe(true);
  });

  it('detects amp query params', () => {
    expect(isAmpUrl('https://example.eu/article?amp=1')).toBe(true);
    expect(isAmpUrl('https://example.eu/article?output=amp')).toBe(true);
  });

  it('detects ampproject CDN subdomain', () => {
    expect(isAmpUrl('https://www-cnn-com.cdn.ampproject.org/c/s/www.cnn.com/sample')).toBe(true);
  });

  it('rejects amputeestore false positive (the canonical regression case)', () => {
    // Legacy Python's whole-string scan matched `/amp` against `//amp` after
    // the scheme. Component-scoped scan says no.
    expect(
      isAmpUrl(
        'https://amputeestore.com/collections/prosthetic-socks/products/knit-rite-liner-liner-sock?variant=4114742017',
      ),
    ).toBe(false);
    expect(
      isAmpUrl('https://amputeestore.com/products/tamarack-glidewear-prosthetic-liner-patch'),
    ).toBe(false);
    expect(isAmpUrl('https://amputeestore.com/products/alps-antiperspirant-spray')).toBe(false);
  });

  it('rejects non-AMP URLs', () => {
    expect(isAmpUrl('https://www.google.com/search?q=foo')).toBe(false);
    expect(isAmpUrl('https://news.ycombinator.com/item?id=42')).toBe(false);
    expect(isAmpUrl('https://en.wikipedia.org/wiki/Wikipedia')).toBe(false);
  });

  it('rejects denylisted domains even with amp in path', () => {
    expect(isAmpUrl('https://www.youtube.com/amp/some-video')).toBe(false);
    expect(isAmpUrl('https://open.spotify.com/amp/track/123')).toBe(false);
    expect(isAmpUrl('https://bandcamp.com/amp/foo')).toBe(false);
  });

  it('rejects malformed URLs', () => {
    expect(isAmpUrl('not a url')).toBe(false);
    expect(isAmpUrl('')).toBe(false);
    expect(isAmpUrl('amp')).toBe(false);
  });
});

describe('isCachedAmp', () => {
  it('detects Google AMP cache', () => {
    expect(
      isCachedAmp('https://www.google.com/amp/s/electrek.co/2018/06/19/tesla-model-3/amp/'),
    ).toBe(true);
    expect(isCachedAmp('https://www.google.co.uk/amp/s/example.eu/article')).toBe(true);
  });

  it('detects Bing AMP cache', () => {
    expect(isCachedAmp('https://www.bing.com/amp/s/example.eu/article')).toBe(true);
  });

  it('detects ampproject domains', () => {
    expect(isCachedAmp('https://cdn.ampproject.org/c/s/example.eu')).toBe(true);
    expect(isCachedAmp('https://www-cnn-com.cdn.ampproject.org/c/s/foo')).toBe(true);
    expect(isCachedAmp('https://example.ampproject.net/some/path')).toBe(true);
  });

  it('rejects publisher AMP pages (not on a third-party cache CDN)', () => {
    expect(isCachedAmp('https://www.bbc.com/news/world-europe/amp')).toBe(false);
    expect(isCachedAmp('https://amp.cnn.com/cnn/2020/article')).toBe(false);
  });
});
