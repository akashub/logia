import { useEffect, useRef, useState, type MutableRefObject } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { DeliveryStatus as Status } from './dictation-types';
export type CapturedTarget = { token: string; status: Status };

export function useDelivery(native: boolean, generation: MutableRefObject<number>, phase: MutableRefObject<string>) {
  const [status, setStatus] = useState<Status>('copy');
  const connected = useRef(false);
  useEffect(() => {
    if (!native) return;
    let disposed = false;
    let unsubscribe: (() => void) | undefined;
    void listen<{ generation: number; status: Status }>('delivery', ({ payload }) => {
      if (!disposed && payload.generation >= generation.current && phase.current !== 'canceling') setStatus(payload.status);
    }).then(off => { if (disposed) off(); else { unsubscribe = off; connected.current = true; } })
      .catch(() => setStatus('copy'));
    return () => { disposed = true; connected.current = false; unsubscribe?.(); };
  }, [native, generation, phase]);

  async function capture(): Promise<CapturedTarget> {
    if (!connected.current) return { token: '0', status: 'copy' };
    return invoke<CapturedTarget>('capture_target');
  }
  async function discard(token: string) {
    if (token !== '0') await invoke('discard_target', { token });
  }
  async function permission() {
    const allowed = await invoke<boolean>('accessibility_permission', { prompt: true });
    setStatus(allowed ? 'copy' : 'permission');
  }
  return { status, setStatus, capture, discard, permission };
}

export function deliveryNote(status: Status, complete: boolean, copied = false) {
  if (status === 'sending') return 'Pasting at your text cursor. Your paragraph stays here.';
  if (status === 'sent') return 'Text sent. Your paragraph stays here for reference.';
  if (status === 'dispatched') return 'Paste sent. Your text is also on the clipboard.';
  if (status === 'clipboard-changed') return 'Your clipboard changed while dictating, so Logia left it alone. Your text is here if you want to copy it.';
  if (status === 'copied') return 'Your text is copied and ready to paste.';
  if (status === 'copy-required') return 'Logia could not prepare this text for automatic insertion. Your paragraph is here to copy.';
  if (status === 'uncertain') return 'Delivery could not be confirmed. Check your field before copying to avoid a duplicate.';
  if (status === 'permission') return 'Typing access is unavailable. Review permissions in Logia Settings. Your text remains here.';
  if (status === 'armed' && !complete) return 'Use the shortcut again to finish. Logia pastes where your text cursor is.';
  // A changed field, unreadable selection and unavailable capabilities can all
  // reach recovery. Do not invent a single cause or claim a failed copy worked.
  if (complete) return copied ? 'The text wasn’t inserted automatically. It’s copied and ready to paste.'
    : 'The text wasn’t inserted automatically. Your paragraph is here to copy.';
  return 'Use the global shortcut from a text field to dictate there, or record here and copy.';
}
