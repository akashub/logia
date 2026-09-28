export type ShortcutMode = 'toggle' | 'hold';
export type ShortcutEvent = { id: string; phase: 'pressed' | 'released'; mode: ShortcutMode };
export type ShortcutRequest = { id: string; mode: ShortcutMode; valid: () => boolean };
