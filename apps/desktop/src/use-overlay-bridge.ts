import { useEffect, useRef } from 'react';
import { emitTo, listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { activePhase, type OverlayAction, type OverlayRequest, type OverlaySnapshot } from './dictation-types';

// claude 2026-09-11: the panel must prove it is showing the current state
// before the microphone opens, and must not silently go dark during capture.
// Contracts in tests/overlay-safety.cjs.
const APPLY_TIMEOUT_MS = 1500;
// While recording, a snapshot that is never acknowledged means the user has no
// visible Stop control. Reveal the settings window rather than leave them stuck.
const LOST_BRIDGE_MS = 1200;

export type OverlayGate = {
  /**
   * Resolves true once the panel has rendered a snapshot published *after*
   * `afterSeq`. Callers capture `publishedSeq()` before changing state, so a
   * stale acknowledgment from the previous snapshot can never satisfy the gate.
   */
  waitForApplied: (afterSeq: number, timeoutMs?: number) => Promise<boolean>;
  /** Sequence number of the most recent published snapshot. */
  publishedSeq: () => number;
  /**
   * Publishes immediately with `overrides` applied, instead of waiting for the
   * next render and effect. Keeps a React commit off the path between the
   * user's gesture and the microphone opening.
   */
  publishNow: (overrides: Partial<OverlaySnapshot>) => Promise<number>;
};

export function useOverlayBridge(native: boolean, snapshot: Omit<OverlaySnapshot, 'seq'>,
  action: (action: OverlayAction, text?: string) => void, onError: (error: string) => void,
  onLostDuringCapture: () => void = () => {}): OverlayGate {
  const current = useRef({ snapshot, action, onError, onLostDuringCapture });
  current.current = { snapshot, action, onError, onLostDuringCapture };
  const connected = useRef(false);
  const seq = useRef(0), applied = useRef(0);
  const waiters = useRef<((ok: boolean) => void)[]>([]);
  const lostTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  function settle(ok: boolean) { waiters.current.splice(0).forEach(resolve => resolve(ok)); }

  function clearLostTimer() {
    if (lostTimer.current) { clearTimeout(lostTimer.current); lostTimer.current = null; }
  }

  async function publish(overrides: Partial<OverlaySnapshot> = {}) {
    const next = ++seq.current;
    await emitTo('overlay', 'overlay-state', { ...current.current.snapshot, ...overrides, seq: next });
    return next;
  }

  useEffect(() => {
    if (!native) return;
    let disposed = false;
    const subscriptions: (() => void)[] = [];
    async function connect() {
      const ready = await listen('overlay-ready', () => {
        if (!disposed) { connected.current = true; void publish().catch(() => {}); }
      });
      if (disposed) { ready(); return; } subscriptions.push(ready);
      const ack = await listen<{ seq: number }>('overlay-applied', ({ payload }) => {
        if (disposed || payload.seq <= applied.current) return;
        applied.current = payload.seq;
        connected.current = true;
        clearLostTimer();
        settle(true);
      });
      if (disposed) { ack(); return; } subscriptions.push(ack);
      const actions = await listen<OverlayRequest>('overlay-action', ({ payload }) => {
        if (!disposed && payload.session === current.current.snapshot.session) current.current.action(payload.action, payload.text);
      });
      if (disposed) { actions(); return; } subscriptions.push(actions);
      await emitTo('overlay', 'overlay-connect');
    }
    void connect().catch(() => { if (!disposed) current.current.onError('Could not connect the overlay. Quit and reopen Logia.'); });
    return () => {
      disposed = true; connected.current = false; clearLostTimer(); settle(false);
      subscriptions.forEach(off => off());
    };
  }, [native]);

  useEffect(() => {
    if (!native) return;
    void publish().catch(() => current.current.onError('Could not update the overlay. Use the shortcut to stop.'));
    // A capture whose panel stops acknowledging leaves the user with no visible
    // Stop. Surface the settings window, which carries working controls.
    if (!activePhase(snapshot.phase)) { clearLostTimer(); return; }
    // Do NOT restart on every snapshot. The elapsed-seconds tick publishes once
    // a second, so a restarting timer would never reach its deadline while
    // recording — the exact case it exists to catch. It is cleared only by an
    // acknowledgment or by leaving the active phase.
    if (lostTimer.current) return;
    lostTimer.current = setTimeout(() => {
      lostTimer.current = null;
      if (applied.current >= seq.current) return;
      current.current.onError('The overlay stopped responding. Use these controls to stop or cancel.');
      current.current.onLostDuringCapture();
      void invoke('show_main_window').catch(() => {});
    }, LOST_BRIDGE_MS);
  }, [native, snapshot]);

  useEffect(() => {
    if (!native || snapshot.phase !== 'ready' || !snapshot.dismissed) return;
    void invoke(snapshot.showIdle ? 'show_overlay' : 'hide_overlay')
      .catch(e => current.current.onError(String(e)));
  }, [native, snapshot.phase, snapshot.dismissed, snapshot.showIdle]);

  return {
    publishedSeq: () => seq.current,
    publishNow: overrides => publish(overrides),
    waitForApplied(afterSeq, timeoutMs = APPLY_TIMEOUT_MS) {
      if (!native) return Promise.resolve(true);
      if (applied.current > afterSeq) return Promise.resolve(true);
      return new Promise<boolean>(resolve => {
        let settled = false;
        const finish = (ok: boolean) => {
          if (settled) return;
          settled = true; clearTimeout(timer);
          waiters.current = waiters.current.filter(w => w !== gate);
          resolve(ok);
        };
        // An acknowledgment of an *older* snapshot must leave this waiter
        // registered. Resolving false there would abandon the session on the
        // first unrelated ack and cost the user a recording.
        const gate = (ok: boolean) => {
          if (!ok) finish(false);
          else if (applied.current > afterSeq) finish(true);
        };
        const timer = setTimeout(() => finish(false), timeoutMs);
        waiters.current.push(gate);
      });
    }
  };
}
