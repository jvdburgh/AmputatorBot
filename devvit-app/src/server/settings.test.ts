// Settings loader tests against a stub SettingsSource — the real
// `@devvit/web/server` settings client is only exercised in `devvit playtest`.

import { describe, expect, it } from 'vitest';
import { loadInstallSettings, SETTING_DEFAULTS, type SettingsSource } from './settings.ts';

function stubSource(values: Record<string, unknown>): SettingsSource {
  return {
    get<T>(name: string) {
      return Promise.resolve(values[name] as T | undefined);
    },
  };
}

describe('loadInstallSettings', () => {
  it('returns defaults when no values are set', async () => {
    const result = await loadInstallSettings(stubSource({}));
    expect(result).toEqual(SETTING_DEFAULTS);
  });

  it('overrides autoReply when explicitly set', async () => {
    const result = await loadInstallSettings(stubSource({ autoReply: false }));
    expect(result.autoReply).toBe(false);
  });

  it('returns customFooter when set to a non-empty string', async () => {
    const result = await loadInstallSettings(
      stubSource({ customFooter: '[Modmail](https://reddit.com/r/foo)' }),
    );
    expect(result.customFooter).toBe('[Modmail](https://reddit.com/r/foo)');
  });

  it('treats empty-string customFooter as undefined', async () => {
    const result = await loadInstallSettings(stubSource({ customFooter: '' }));
    expect(result.customFooter).toBeUndefined();
  });
});
