import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export function useDictationWindow(native: boolean, onShortcut: () => void, onDismiss: () => void) {
  const [floating, setFloating] = useState(false);
  const [changing, setChanging] = useState(false);
  const [shortcut, setShortcut] = useState('');
  const [shortcutError, setShortcutError] = useState('');
  const [retryable, setRetryable] = useState(false);
  const handler = useRef(onShortcut);
  handler.current = onShortcut;
  const dismiss = useRef(onDismiss);
  dismiss.current = onDismiss;
  const busy = useRef(false);
  useLayoutEffect(() => { window.scrollTo(0, 0); }, [floating]);

  async function register() {
    try { setShortcut(await invoke<string>('register_shortcut')); setShortcutError(''); setRetryable(false); }
    catch (e) { setShortcut(''); setShortcutError(String(e)); setRetryable(true); await invoke('show_main_window').catch(() => {}); }
  }
  useEffect(() => {
    if (!native) return;
    let disposed = false;
    const subscriptions: (() => void)[] = [];
    async function connect() {
      for (const [name, callback] of [['dictation-shortcut', () => handler.current()], ['dictation-dismiss', () => dismiss.current()]] as const) {
        const unsub = await listen(name, () => { if (!disposed) callback(); });
        if (disposed) { unsub(); return; }
        subscriptions.push(unsub);
      }
      await register();
    }
    void connect().catch(() => {
      subscriptions.splice(0).forEach(unsub => unsub());
      if (disposed) return;
      setRetryable(false);
      setShortcutError('Could not connect the shortcut. Quit and reopen Logia.');
      void invoke('show_main_window').catch(() => {});
    });
    return () => { disposed = true; subscriptions.forEach(unsub => unsub()); };
  }, [native]);

  async function change(next: boolean, reveal = false) {
    if (busy.current) return false;
    busy.current = true; setChanging(true);
    try {
      await invoke('set_floating', { floating: next, reveal });
      setFloating(next);
      return true;
    } finally { busy.current = false; setChanging(false); }
  }
  return { floating, changing, shortcut, shortcutError, retryable, register, change };
}
