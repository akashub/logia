import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export function useDictationWindow(native: boolean, shortcut: string, onShortcut: () => void, onDismiss: () => void) {
  const [label, setLabel] = useState(''), [error, setError] = useState('');
  const [retryable, setRetryable] = useState(false);
  const handlers = useRef({ onShortcut, onDismiss, shortcut });
  handlers.current = { onShortcut, onDismiss, shortcut };
  async function register(next = handlers.current.shortcut) {
    try {
      const result = await invoke<string>('register_shortcut', { shortcut: next });
      setLabel(result); setError(''); setRetryable(false);
    } catch (e) { setError(String(e)); setRetryable(true); throw e; }
  }
  useEffect(() => {
    if (!native) return;
    let disposed = false;
    const subscriptions: (() => void)[] = [];
    async function connect() {
      for (const [name, callback] of [
        ['dictation-shortcut', () => handlers.current.onShortcut()],
        ['dictation-dismiss', () => handlers.current.onDismiss()]
      ] as const) {
        const unsubscribe = await listen(name, () => { if (!disposed) callback(); });
        if (disposed) { unsubscribe(); return; }
        subscriptions.push(unsubscribe);
      }
      await register().catch(() => invoke('show_main_window').catch(() => {}));
    }
    void connect().catch(() => {
      subscriptions.splice(0).forEach(unsubscribe => unsubscribe());
      if (!disposed) {
        setRetryable(false); setError('Could not connect the shortcut. Quit and reopen Logia.');
        void invoke('show_main_window').catch(() => {});
      }
    });
    return () => { disposed = true; subscriptions.forEach(unsubscribe => unsubscribe()); };
  }, [native]);
  return { shortcut: label, shortcutError: error, retryable, register };
}
