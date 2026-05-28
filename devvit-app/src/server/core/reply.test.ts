// Reply markdown — same scenarios as `backend/src/reply.rs#tests`, plus
// snapshots for the singular + plural variants so format drift is loud.

import { describe, expect, it } from 'vitest';
import type { Canonical, Link, UrlMeta } from '../backend/types.ts';
import { buildReply } from './reply.ts';

function ampOrigin(url: string, cached: boolean): UrlMeta {
  return {
    url,
    domain: 'example',
    isAmp: true,
    isCached: cached,
    isValid: true,
  };
}

function canonical(url: string, domain: string, isAmp: boolean): Canonical {
  return {
    url,
    domain,
    isAmp,
    isCached: isAmp ? false : null,
    isValid: true,
    isAlt: false,
    type: 'REL',
    urlSimilarity: 1.0,
  };
}

describe('buildReply', () => {
  it('returns null when no canonical was found', () => {
    const link: Link = {
      origin: ampOrigin('https://www.google.com/amp/s/example.eu/x', false),
      canonical: null,
      canonicals: [],
      ampCanonical: null,
    };
    expect(buildReply([link], { triggerType: 'comment' })).toBeNull();
  });

  it('returns null when origin is not AMP', () => {
    const link: Link = {
      origin: { ...ampOrigin('https://example.eu/x', false), isAmp: false },
      canonical: canonical('https://other.example/y', 'other', false),
      canonicals: [],
      ampCanonical: null,
    };
    expect(buildReply([link], { triggerType: 'comment' })).toBeNull();
  });

  it('uses singular phrasing + comment voice ("you shared") for one canonical', () => {
    const link: Link = {
      origin: ampOrigin('https://www.google.com/amp/s/example.eu/article', false),
      canonical: canonical('https://example.eu/article', 'example', false),
      canonicals: [],
      ampCanonical: null,
    };
    const reply = buildReply([link], { triggerType: 'comment' });
    expect(reply).toContain('It looks like you shared an AMP link.');
    expect(reply).toContain('the canonical page');
    expect(reply).toContain('**[https://example.eu/article](https://example.eu/article)**');
    expect(reply).toContain(
      "AMP is supposed to be faster, but it's controversial because of [concerns over privacy and the Open Web]",
    );
    // No legacy beep-boop or summon link.
    expect(reply).not.toContain("I'm a bot");
    expect(reply).not.toContain('Summon: u/AmputatorBot');
  });

  it('uses plural phrasing + post voice ("OP posted") for multiple canonicals', () => {
    const links: Link[] = [
      {
        origin: ampOrigin('https://www.google.com/amp/s/example.eu/a', false),
        canonical: canonical('https://example.eu/a', 'example', false),
        canonicals: [],
        ampCanonical: null,
      },
      {
        origin: ampOrigin('https://www.google.com/amp/s/example.eu/b', false),
        canonical: canonical('https://example.eu/b', 'example', false),
        canonicals: [],
        ampCanonical: null,
      },
    ];
    const reply = buildReply(links, { triggerType: 'post' });
    expect(reply).toContain('It looks like OP posted some AMP links.');
    expect(reply).toContain('the canonical pages');
    expect(reply).toContain('- **[https://example.eu/a]');
    expect(reply).toContain('- **[https://example.eu/b]');
  });

  it('appends the cached-AMP note when origin was cached', () => {
    const link: Link = {
      origin: ampOrigin('https://www.google.com/amp/s/example.eu/x', true),
      canonical: canonical('https://example.eu/x', 'example', false),
      canonicals: [],
      ampCanonical: null,
    };
    const reply = buildReply([link], { triggerType: 'comment' });
    expect(reply).toContain('Fully cached AMP pages (like the one you shared)');
  });

  it('surfaces a cross-domain alt canonical', () => {
    const primary = canonical('https://example.eu/x', 'example', false);
    const alt = canonical('https://syndicated.partner.example/x', 'syndicated', false);
    const link: Link = {
      origin: ampOrigin('https://www.google.com/amp/s/example.eu/x', false),
      canonical: primary,
      canonicals: [primary, alt],
      ampCanonical: null,
    };
    const reply = buildReply([link], { triggerType: 'comment' });
    expect(reply).toContain(
      'Syndicated canonical: **[https://syndicated.partner.example/x](https://syndicated.partner.example/x)**',
    );
  });

  it('falls back to amp_canonical when no real canonical is available', () => {
    const link: Link = {
      origin: ampOrigin('https://www.google.com/amp/s/example.eu/dead-end', true),
      canonical: null,
      canonicals: [],
      ampCanonical: canonical('https://example.eu/dead-end/amp/', 'example', true),
    };
    const reply = buildReply([link], { triggerType: 'comment' });
    expect(reply).toContain(
      '**[https://example.eu/dead-end/amp/](https://example.eu/dead-end/amp/)** ^(Still AMP, but no longer cached - unable to process further)',
    );
  });

  it('appends the customFooter inside the superscript group when set', () => {
    const link: Link = {
      origin: ampOrigin('https://www.google.com/amp/s/example.eu/x', false),
      canonical: canonical('https://example.eu/x', 'example', false),
      canonicals: [],
      ampCanonical: null,
    };
    const reply = buildReply([link], {
      triggerType: 'comment',
      customFooter: '[Modmail us](https://reddit.com/r/Subreddit)',
    });
    expect(reply).toContain('^( | )[Modmail us](https://reddit.com/r/Subreddit))');
  });

  it('matches the locked snapshot for a typical singular comment reply', () => {
    const link: Link = {
      origin: ampOrigin('https://www.google.com/amp/s/example.eu/article', true),
      canonical: canonical('https://example.eu/article', 'example', false),
      canonicals: [],
      ampCanonical: null,
    };
    expect(buildReply([link], { triggerType: 'comment' })).toMatchInlineSnapshot(`
      "It looks like you shared an AMP link. AMP is supposed to be faster, but it's controversial because of [concerns over privacy and the Open Web](https://www.reddit.com/r/AmputatorBot/comments/ehrq3z/why_did_i_build_amputatorbot). Fully cached AMP pages (like the one you shared), are [especially problematic](https://www.reddit.com/r/AmputatorBot/comments/ehrq3z/why_did_i_build_amputatorbot).

      Maybe check out **the canonical page** instead: **[https://example.eu/article](https://example.eu/article)**

      *****

       ^([Why & About](https://www.reddit.com/r/AmputatorBot/comments/ehrq3z/why_did_i_build_amputatorbot)^( | )[r/AmputatorBot](https://reddit.com/r/AmputatorBot)^( | )[Source](https://github.com/KilledMufasa/AmputatorBot))"
    `);
  });
});
