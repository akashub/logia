import { useEffect, useRef, useState, type MutableRefObject } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

type Status = 'copy' | 'permission' | 'armed' | 'sending' | 'sent' | 'uncertain';
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

export function deliveryNote(status: Status, complete: boolean) {
  if (status === 'sending') return 'Sending your words to the original field. Your paragraph stays here.';
  if (status === 'sent') return 'Text sent to your original field. Your paragraph stays here for reference.';
  if (status === 'uncertain') return 'Delivery could not be confirmed. Check your field before copying to avoid a duplicate.';
  if (status === 'permission') return 'Enable Logia in macOS Accessibility to type into other apps. Your text is available to copy.';
  if (status === 'armed' && !complete) return 'Use the shortcut again to stop and send to this field. Keep your cursor in place.';
  if (complete) return 'Copy text to use it in your app. Your paragraph stays here until you clear it or start again.';
  return 'Use the global shortcut from a text field to dictate there, or record here and copy.';
}
