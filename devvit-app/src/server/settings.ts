// Per-install settings loader.
//
// Settings are declared in `devvit.json` under `settings.subreddit` and
// configured by each mod when installing the app. We re-read on every
// trigger to keep state fresh — the read is cheap and mods can toggle
// `autoReply` mid-incident without restarting anything.
//
// Schema (must stay in sync with devvit.json):
//   - autoReply (bool, default true)   — single on/off toggle for the bot
//   - customFooter (string, optional)  — appended to the reply footer
//
// `settings.get<T>` from `@devvit/web/server` returns `T | undefined`. We
// fall back to the defaults declared here so the trigger handler always
// has concrete values to work with, even in edge cases where a setting
// hasn't yet been written to the install (e.g. first-time invocation
// before the mod has opened the settings page).

import { type SettingsClient, settings } from '@devvit/web/server';

export type InstallSettings = {
  autoReply: boolean;
  customFooter: string | undefined;
};

export const SETTING_DEFAULTS: InstallSettings = {
  autoReply: true,
  customFooter: undefined,
};

// Just the `get` method of the real `SettingsClient` — lets tests inject a
// minimal stub without standing up the full client.
export type SettingsSource = Pick<SettingsClient, 'get'>;

export async function loadInstallSettings(
  source: SettingsSource = settings,
): Promise<InstallSettings> {
  const [autoReply, customFooter] = await Promise.all([
    source.get<boolean>('autoReply'),
    source.get<string>('customFooter'),
  ]);
  return {
    autoReply: autoReply ?? SETTING_DEFAULTS.autoReply,
    // Treat empty string as "not set" — the install form accepts blank as
    // the way to clear the value, and an empty customFooter would render
    // an awkward ` | ` with nothing after it in the reply footer.
    customFooter: customFooter && customFooter.length > 0 ? customFooter : undefined,
  };
}
