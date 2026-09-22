import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export type MicrophoneStatus = 'unknown' | 'not_determined' | 'denied' | 'restricted' | 'authorized' | 'unsupported';
export type PermissionKind = 'microphone' | 'accessibility';
type Snapshot = { microphone: MicrophoneStatus; accessibility: 'unknown' | 'denied' | 'authorized' };
const initial: Snapshot = { microphone: 'unknown', accessibility: 'unknown' };
const granted = (s: Snapshot) => s.microphone === 'authorized' && s.accessibility === 'authorized';

async function read() {
    const [mic, ax] = await Promise.allSettled([
      invoke<MicrophoneStatus>('microphone_status'),
      invoke<boolean>('accessibility_permission', { prompt: false })
    ]);
    const status: Snapshot = { microphone: mic.status === 'fulfilled' ? mic.value : 'unknown',
      accessibility: ax.status === 'fulfilled' ? ax.value ? 'authorized' : 'denied' : 'unknown' };
    return { status, error: mic.status === 'rejected' || ax.status === 'rejected'
      ? 'Permission status unavailable. Check permissions again.' : '' };
}

export function usePermissions(native: boolean) {
  const [status, setStatus] = useState(initial), [setup, setSetup] = useState(false);
  const [busy, setBusy] = useState(false), [actionError, setError] = useState('');
  const [readError, setReadError] = useState('');
  const revision = useRef(0), acting = useRef(false), mounted = useRef(true);
  const inFlight = useRef<Promise<Snapshot | null> | null>(null);
  const publish = (next: Snapshot) => {
    setStatus(next);
    if (!granted(next)) setSetup(true);
  };
  const refresh = useCallback((): Promise<Snapshot | null> => {
    if (!native || acting.current) return Promise.resolve(null);
    // Focus, native activation, polling and the shortcut share one current read.
    // A second caller must not invalidate the first caller's authorization gate.
    if (inFlight.current) return inFlight.current;
    const ticket = revision.current;
    const pending = read().then(next => {
      if (!mounted.current || ticket !== revision.current) return null;
      publish(next.status); setReadError(next.error);
      return next.status;
    }).finally(() => { if (inFlight.current === pending) inFlight.current = null; });
    inFlight.current = pending;
    return pending;
  }, [native]);

  useEffect(() => {
    mounted.current = true;
    if (!native) return;
    let off: (() => void) | undefined, disposed = false;
    void refresh();
    const focus = () => { if (document.visibilityState === 'visible') void refresh(); };
    window.addEventListener('focus', focus);
    document.addEventListener('visibilitychange', focus);
    // WKWebView DOM focus is not evidence of native app activation. Use both.
    void listen('permissions-refresh', focus).then(unsubscribe => {
      if (disposed) unsubscribe(); else off = unsubscribe;
    }).catch(() => {});
    return () => {
      disposed = true; mounted.current = false; ++revision.current; inFlight.current = null; off?.();
      window.removeEventListener('focus', focus);
      document.removeEventListener('visibilitychange', focus);
    };
  }, [native, refresh]);
  useEffect(() => {
    if (!native || !setup) return;
    const timer = setInterval(() => {
      if (document.visibilityState === 'visible' && document.hasFocus()) void refresh();
    }, 1500);
    return () => clearInterval(timer);
  }, [native, setup, refresh]);

  async function act(kind: PermissionKind) {
    if (!native || acting.current) return;
    acting.current = true; ++revision.current; inFlight.current = null; setBusy(true); setError('');
    try {
      if (kind === 'microphone') {
        let mic = await invoke<MicrophoneStatus>('microphone_status');
        if (mic === 'not_determined') mic = await invoke<MicrophoneStatus>('request_microphone');
        if (mic === 'denied') await invoke('open_permission_settings', { kind });
        else if (mic === 'restricted') throw Error('Microphone access is restricted on this Mac. Check Screen Time or your administrator’s settings.');
        else if (mic !== 'authorized') throw Error('Could not read microphone permission. Try again.');
      } else {
        const allowed = await invoke<boolean>('accessibility_permission', { prompt: true });
        // One action requests access and takes the user to the actual switch.
        if (!allowed) await invoke('open_permission_settings', { kind });
      }
    } catch (e) { if (mounted.current) setError(String(e)); }
    finally {
      const next = await read();
      if (mounted.current) { publish(next.status); setReadError(next.error); setBusy(false); }
      acting.current = false;
    }
  }
  function finish() { if (granted(status)) { setSetup(false); setError(''); } }
  return { ...status, setup, busy, error: readError || actionError, refresh, act, finish,
    review: () => { setSetup(true); void refresh(); } };
}
export type Permissions = ReturnType<typeof usePermissions>;
