import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useDelivery, type CapturedTarget } from './use-delivery';
import { useRecognition } from './use-recognition';
import { activePhase, type Phase } from './dictation-types';
import type { OverlayGate } from './use-overlay-bridge';
import type { MutableRefObject } from 'react';

// How long the in-app voice test waits for a stalled panel reveal before
// relying on the panel's own acknowledgment instead.
const REVEAL_STALL_MS = 900;

// Only the persistent settings webview instantiates this controller.
// `overlayGate` is filled in by main.tsx after the bridge is created; it is
// only read inside async handlers, never during render.
export function useDictation(native: boolean, overlayGate?: MutableRefObject<OverlayGate | null>) {
  const [phase, setPhase] = useState<Phase>(native ? 'checking' : 'setup');
  const [text, setText] = useState(''), [error, setError] = useState('');
  const [progress, setProgress] = useState(0), [seconds, setSeconds] = useState(0);
  const [copied, setCopied] = useState(false), [complete, setComplete] = useState(false);
  const [dismissed, setDismissed] = useState(true), [isDismissing, setDismissing] = useState(false);
  const [level, setLevel] = useState(0), [heardAudio, setHeardAudio] = useState(false);
  const generation = useRef(0), controlIntent = useRef(0), recordingIntent = useRef(0);
  const shortcutStarting = useRef(false), dismissing = useRef(false);
  const phaseRef = useRef(phase); phaseRef.current = phase;
  const delivery = useDelivery(native, generation, phaseRef);
  function transition(next: Phase) { phaseRef.current = next; setPhase(next); }
  useRecognition({ native, generation, phase: phaseRef, transition, text: setText, error: setError,
    complete: setComplete, seconds: setSeconds, progress: setProgress, prepare,
    level: peak => { setLevel(peak); if (peak > 0.0005) setHeardAudio(true); } });
  // claude 2026-09-11: silence for several seconds of an active recording is
  // almost always a microphone permission that no longer applies — macOS gives
  // a denied app silent buffers instead of an error. Say so rather than letting
  // it look like the recognizer has nothing to say.
  const deaf = phase === 'recording' && seconds >= 4 && !heardAudio;
  useEffect(() => {
    if (phase !== 'recording') return;
    const timer = setInterval(() => setSeconds(value => value + 1), 1000);
    return () => clearInterval(timer);
  }, [phase]);
  useEffect(() => { if (phase === 'recording' && seconds >= 60) void stop(); }, [seconds, phase]);
  useEffect(() => {
    if (phase !== 'ready' || !complete || !['sent', 'dispatched'].includes(delivery.status) || dismissed) return;
    const timer = setTimeout(() => setDismissed(true), 700);
    return () => clearTimeout(timer);
  }, [phase, complete, delivery.status, dismissed]);
  // claude 2026-09-11: when the target could not be typed into, put the text on
  // the clipboard without being asked. The user has already stopped speaking and
  // wants the words somewhere useful; making them click Copy first is a step for
  // nothing. Delivery that succeeded needs no clipboard write.
  // Once per session, not per keystroke: later edits leave the clipboard alone
  // so Copy stays a live affordance the user can see and press again.
  const autoCopied = useRef(0);
  useEffect(() => {
    // Both fallbacks reach the clipboard: 'copy' (no usable target) and
    // 'permission' (macOS refused). Either way the user needs to paste.
    if (phase !== 'ready' || !complete || !text) return;
    if (delivery.status === 'copied') {
      if (autoCopied.current !== recordingIntent.current) {
        autoCopied.current = recordingIntent.current;
        setCopied(true);
      }
      return;
    }
    if (delivery.status !== 'copy' && delivery.status !== 'permission') return;
    if (autoCopied.current === recordingIntent.current) return;
    autoCopied.current = recordingIntent.current;
    let cancelled = false;
    void invoke('plugin:clipboard-manager|write_text', { text })
      .then(() => { if (!cancelled) setCopied(true); })
      .catch(() => { if (!cancelled) setError('Automatic copy failed. Use Copy to try again; your text is still here.'); });
    return () => { cancelled = true; };
  }, [phase, complete, text, delivery.status]);

  async function toggleFromShortcut() {
    if (dismissing.current) return;
    if (phaseRef.current === 'recording') { await stop(); return; }
    if (phaseRef.current !== 'ready' || shortcutStarting.current) return;
    shortcutStarting.current = true;
    const intent = controlIntent.current;
    let target: CapturedTarget | undefined, started = false;
    try {
      target = await delivery.capture();
      if (controlIntent.current !== intent) return;
      started = await start(target);
    } catch (e) { setError(String(e)); }
    finally {
      if (target && !started) await delivery.discard(target.token).catch(() => {});
      shortcutStarting.current = false;
    }
  }
  async function dismissWindow() {
    if (dismissing.current) return;
    controlIntent.current++;
    if (activePhase(phaseRef.current)) { setError('Stop or cancel recording before dismissing Logia.'); return; }
    dismissing.current = true; setDismissing(true);
    try { await invoke('hide_main_window'); setError(''); }
    catch (e) { setError(String(e)); }
    finally { dismissing.current = false; setDismissing(false); }
  }
  function dismissOverlay() {
    controlIntent.current++;
    if (activePhase(phaseRef.current)) { setError('Stop or cancel recording before dismissing Logia.'); return; }
    setDismissed(true); setError('');
  }
  async function start(target?: CapturedTarget): Promise<boolean> {
    if (!native || phaseRef.current !== 'ready' || dismissing.current) return false;
    // claude 2026-09-11: the microphone must not open until the panel confirms
    // it is showing this session. Three rules make that safe:
    //
    //  1. Reveal BEFORE moving to `loading`, so a window server that never
    //     answers leaves the session at `ready` instead of stranding it.
    //  2. A reveal that *refuses* is always final — no recording. A reveal that
    //     merely stalls depends on the entry point: global dictation happens
    //     inside another application where the panel is the only Stop control,
    //     so it waits. The voice test runs in this window, which already
    //     carries Stop and Cancel, so it proceeds on the acknowledgment alone.
    //  3. Read the published sequence before the state change, so a stale
    //     acknowledgment of the previous snapshot cannot satisfy the gate.
    const gate = overlayGate?.current;
    if (gate) {
      let refused = '';
      const reveal = invoke('show_overlay').catch(e => { refused = String(e); });
      if (target) await reveal;
      else await Promise.race([reveal, new Promise(resolve => setTimeout(resolve, REVEAL_STALL_MS))]);
      if (refused) { setError(refused); return false; }
      if (phaseRef.current !== 'ready' || dismissing.current) return false;
    }
    controlIntent.current++; recordingIntent.current++;
    const intent = controlIntent.current;
    const baseline = gate ? gate.publishedSeq() : 0;
    setError(''); setText(''); setComplete(false); setCopied(false); setSeconds(0); setLevel(0); setHeardAudio(false); setDismissed(false); transition('loading');
    delivery.setStatus(target?.status ?? 'copy');
    if (gate) {
      // Publish the armed state straight away rather than waiting for React to
      // commit and the effect to fire: that round trip sits directly between
      // the user's gesture and the microphone opening.
      await gate.publishNow({ session: recordingIntent.current, phase: 'loading', dismissed: false,
        text: '', complete: false, error: '', seconds: 0, delivery: target?.status ?? 'copy' }).catch(() => {});
      const acknowledged = await gate.waitForApplied(baseline);
      if (!acknowledged) {
        setError('The recording overlay did not appear, so nothing was recorded. Use these controls, or reopen Logia from the menu bar.');
        await invoke('show_main_window').catch(() => {});
        delivery.setStatus('copy'); setDismissed(true); transition('ready');
        return false;
      }
      if (controlIntent.current !== intent) { delivery.setStatus('copy'); transition('ready'); return false; }
    }
    try {
      generation.current = Math.max(generation.current, await invoke<number>('start_recording', { target: target?.token ?? null }));
      return true;
    } catch (e) { setError(String(e)); delivery.setStatus('copy'); transition('ready'); return false; }
  }
  async function stop() {
    if (phaseRef.current !== 'recording') return;
    controlIntent.current++; setError(''); transition('finishing');
    try { await invoke('stop_recording'); } catch (e) { setError(String(e)); }
  }
  async function cancel() {
    controlIntent.current++;
    const intent = recordingIntent.current;
    const preparing = phaseRef.current === 'warming';
    transition('canceling');
    try {
      const outcome = await invoke<{ status: string; text?: string }>('cancel_recording');
      if (recordingIntent.current !== intent) return;
      setError('');
      if (outcome?.status === 'sent' || outcome?.status === 'dispatched' || outcome?.status === 'copied' || outcome?.status === 'uncertain') {
        delivery.setStatus(outcome.status); setText(outcome.text ?? ''); setComplete(true); setDismissed(false);
      } else {
        delivery.setStatus('copy');
        if (!preparing) { setText(''); setComplete(false); }
        setDismissed(true);
      }
      transition('ready');
    } catch (e) { setError(String(e)); }
  }
  async function download() {
    if (phaseRef.current !== 'setup') return;
    transition('downloading'); setError(''); setProgress(0);
    try { await invoke('download_model'); await prepare(); }
    catch (e) { setError(String(e)); transition('setup'); }
  }
  async function prepare() {
    if (activePhase(phaseRef.current) || phaseRef.current === 'warming') return;
    transition('warming');
    try { generation.current = Math.max(generation.current, await invoke<number>('warmup_recognizer')); }
    catch (e) { setError(String(e)); transition('ready'); }
  }
  async function copy() {
    if (!text || activePhase(phaseRef.current)) return;
    try { await invoke('plugin:clipboard-manager|write_text', { text }); setCopied(true); }
    catch { setError('Could not copy. Open the editor and copy your text there.'); }
  }
  function edit(value: string) {
    autoCopied.current = recordingIntent.current;
    setText(value); setCopied(false); delivery.setStatus('copy');
  }
  function clear() { if (!activePhase(phaseRef.current)) { edit(''); setComplete(false); setDismissed(true); } }
  return { phase, text, error, progress, seconds, copied, complete, dismissed, isDismissing, level, deaf,
    session: recordingIntent.current, delivery, active: activePhase(phase), setError,
    toggleFromShortcut, dismissWindow, dismissOverlay, start, stop, cancel, download, prepare, copy, edit, clear };
}
