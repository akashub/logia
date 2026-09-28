import { useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { useDictation } from './use-dictation';
import type { Permissions } from './use-permissions';
import type { ShortcutEvent } from './shortcut-types';

export function useShortcutControl(session: ReturnType<typeof useDictation>, permissions: Permissions) {
  const pending = useRef(false);
  const held = useRef<{ id: string; released: boolean } | null>(null);
  const queued = useRef<ShortcutEvent | null>(null);
  const last = useRef('');
  function invalidate() {
    if (held.current) held.current.released = true;
    held.current = null; queued.current = null;
  }
  useEffect(() => invalidate, []);

  function handle(event: ShortcutEvent) {
    if (!event || typeof event.id !== 'string' || !['toggle', 'hold'].includes(event.mode)) return;
    if (event.phase === 'released') {
      if (event.mode === 'hold' && held.current?.id === event.id) {
        held.current.released = true;
        if (queued.current?.id === event.id) queued.current = null;
        void session.releaseShortcut(event.id);
      }
      return;
    }
    if (event.phase !== 'pressed' || event.id === last.current) return;
    last.current = event.id;
    if (event.mode === 'toggle' && session.currentPhase() === 'recording') { void session.stop(); return; }
    if (session.currentPhase() !== 'ready') return;
    if (pending.current) {
      // Keep one still-held replacement while the released request unwinds.
      if (event.mode === 'hold' && held.current?.released) {
        held.current = { id: event.id, released: false };
        queued.current = event;
      }
      return;
    }
    begin(event);
  }

  function begin(event: ShortcutEvent) {
    const gesture = { id: event.id, released: false };
    held.current = gesture; pending.current = true;
    const valid = () => held.current === gesture && !gesture.released;
    void (async () => {
      try {
        const status = await permissions.refresh();
        if (!valid() || !status || session.currentPhase() !== 'ready') return;
        if (status.microphone !== 'authorized' || status.accessibility !== 'authorized') {
          permissions.review(); await invoke('show_main_window'); return;
        }
        await session.startFromShortcut({ id: event.id, mode: event.mode, valid });
      } catch (error) { if (valid()) session.setError(String(error)); }
      finally {
        pending.current = false;
        const next = queued.current; queued.current = null;
        if (next && held.current?.id === next.id && !held.current.released && session.currentPhase() === 'ready') begin(next);
      }
    })();
  }
  return { handle, invalidate };
}
