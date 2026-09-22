export type Preferences = {
  position: 'top' | 'bottom';
  showIdle: boolean;
  theme: 'light' | 'dark';
  shortcut: string;
};
type PreferenceStorage = Pick<Storage, 'getItem' | 'setItem'>;
const key = 'logia.preferences.v1';
// claude 2026-09-11: the panel appears when you ask for it. A dot sitting on
// screen all day is ambient clutter, not feedback.
const defaults: Preferences = { position: 'bottom', showIdle: false, theme: 'light', shortcut: 'Control+Alt+Space' };
const supportedShortcuts = ['Control+Alt+Space', 'Control+Shift+Space', 'Alt+Shift+Space'];

function validated(value: unknown): Preferences {
  const fields = value && typeof value === 'object' && !Array.isArray(value) ? value as Record<string, unknown> : {};
  const shortcut = typeof fields.shortcut === 'string' && supportedShortcuts.includes(fields.shortcut)
    ? fields.shortcut : defaults.shortcut;
  return {
    position: fields.position === 'top' ? 'top' : 'bottom',
    showIdle: typeof fields.showIdle === 'boolean' ? fields.showIdle : defaults.showIdle,
    theme: fields.theme === 'dark' ? 'dark' : 'light',
    shortcut,
  };
}

export function readPreferences(storage?: PreferenceStorage): Preferences {
  try { return validated(JSON.parse((storage ?? globalThis.localStorage).getItem(key) ?? 'null')); }
  catch { return { ...defaults }; }
}

export function savePreferences(preferences: Preferences, storage?: PreferenceStorage) {
  try { (storage ?? globalThis.localStorage).setItem(key, JSON.stringify(validated(preferences))); }
  catch { /* Preferences still apply for this app session if storage is unavailable. */ }
}
