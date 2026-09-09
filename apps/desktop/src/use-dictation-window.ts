import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export function useDictationWindow(native: boolean, onShortcut: () => void) {
  const [floating, setFloating] = useState(false);
  const [changing, setChanging] = useState(false);
  const [shortcut, setShortcut] = useState('');
  const [shortcutError, setShortcutError] = useState('');
  const handler = useRef(onShortcut);
  handler.current = onShortcut;
  const busy = useRef(false);
  useLayoutEffect(() => { window.scrollTo(0, 0); }, [floating]);

  async function register() {
    try { setShortcut(await invoke<string>('register_shortcut')); setShortcutError(''); }
    catch (e) { setShortcut(''); setShortcutError(String(e)); }
  }
  useEffect(() => {
    if (!native) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void listen('dictation-shortcut', () => { if (!disposed) handler.current(); }).then(unsub => {
      if (disposed) { unsub(); return; }
      unlisten = unsub;
      void register();
    }).catch(() => setShortcutError('Could not connect the shortcut. Close and reopen Logia.'));
    return () => { disposed = true; unlisten?.(); };
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
  return { floating, changing, shortcut, shortcutError, register, change };
}
