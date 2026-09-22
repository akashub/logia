import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { emitTo, listen } from '@tauri-apps/api/event';
import { usePresentedTranscript } from './use-presented-transcript';
import { activePhase, type OverlayAction, type OverlaySnapshot } from './dictation-types';
import { deliveryNote } from './use-delivery';
import { useOverlayEditing } from './use-overlay-editing';
import './overlay.css';

const initial: OverlaySnapshot = { seq: 0, session: 0, phase: 'checking', text: '', complete: false, error: '',
  seconds: 0, delivery: 'copy', copied: false, dismissed: true, shortcut: '', position: 'bottom', showIdle: true,
  level: 0, deaf: false };

export function Overlay() {
  const [state, setState] = useState(initial), [connectionError, setConnectionError] = useState('');
  const [expanded, setExpanded] = useState(false), [following, setFollowing] = useState(true);
  const [height, setHeight] = useState(24), [outcomeHeight, setOutcomeHeight] = useState(36);
  const transcript = useRef<HTMLTextAreaElement>(null), outcome = useRef<HTMLParagraphElement>(null);
  const maximum = useRef(24), followRef = useRef(true), sawWords = useRef(false), warningSeen = useRef(false);
  const pendingSize = useRef<{ width: number; height: number; position: string } | null>(null), resizing = useRef(false);
  const active = activePhase(state.phase);
  const idle = state.dismissed && !active && state.phase !== 'warming';
  const shown = usePresentedTranscript(state.text, (state.phase === 'recording' || state.phase === 'finishing') && !state.complete);
  const delivered = state.delivery === 'sent' || state.delivery === 'dispatched';
  const recovery = !idle && state.phase === 'ready' && (!delivered || Boolean(state.error));
  const editor = useOverlayEditing(recovery, state.session);
  useEffect(() => { if (editor.editing) transcript.current?.focus(); }, [editor.editing]);
  const success = !idle && state.phase === 'ready' && delivered && !recovery;
  const rung = idle ? 'idle' : (state.text || sawWords.current || recovery) ? 'speaking' : 'armed';
  const label = state.phase === 'warming' || state.phase === 'loading' ? 'Preparing' : state.phase === 'recording' ? 'Listening'
    : state.phase === 'finishing' ? 'Finishing' : state.phase === 'canceling' ? 'Stopping' : success ? (state.delivery === 'dispatched' ? 'Paste sent' : 'Text sent') : recovery ? 'Text ready' : 'Idle';
  const compactSpeaking = rung === 'speaking' && !expanded && !recovery && !state.deaf && state.text.length < 48 && state.seconds < 5;
  const transcriptLimit = compactSpeaking ? 24 : (expanded ? 120 : 48);
  const action = (action: OverlayAction, text?: string) => void emitTo('main', 'overlay-action', { action, text, session: state.session })
    .catch(() => setConnectionError('Use the shortcut to stop. Reopen Logia from the menu bar.'));

  useEffect(() => {
    let disposed = false;
    const subscriptions: (() => void)[] = [];
    async function connect() {
      const off = await listen<OverlaySnapshot>('overlay-state', ({ payload }) => { if (!disposed) setState(payload); });
      if (disposed) { off(); return; } subscriptions.push(off);
      const reconnect = await listen('overlay-connect', () => void emitTo('main', 'overlay-ready'));
      if (disposed) { reconnect(); return; } subscriptions.push(reconnect);
      await emitTo('main', 'overlay-ready');
    }
    void connect().catch(() => setConnectionError('Overlay unavailable. Open Logia from the menu bar.'));
    return () => { disposed = true; subscriptions.forEach(off => off()); };
  }, []);
  // claude 2026-09-11: acknowledge after commit, not on receipt. The main
  // window blocks the microphone until this arrives, so it must mean "this
  // snapshot is on screen" rather than "the event was delivered".
  useEffect(() => {
    if (state.seq === 0) return;
    void emitTo('main', 'overlay-applied', { seq: state.seq, session: state.session })
      .catch(() => setConnectionError('Use the shortcut to stop. Reopen Logia from the menu bar.'));
  }, [state.seq, state.session]);
  useLayoutEffect(() => {
    maximum.current = 24; sawWords.current = false; warningSeen.current = false; setExpanded(false); followRef.current = true; setFollowing(true); setHeight(24);
  }, [state.session]);
  useLayoutEffect(() => {
    const node = transcript.current;
    if (!node || rung !== 'speaking') return;
    if (state.text) sawWords.current = true;
    const previous = node.scrollTop;
    node.style.height = 'auto';
    maximum.current = Math.max(maximum.current, Math.min(transcriptLimit, node.scrollHeight));
    node.style.height = `${maximum.current}px`;
    node.scrollTop = followRef.current ? node.scrollHeight : previous;
    setHeight(maximum.current);
    if (outcome.current) setOutcomeHeight(outcome.current.offsetHeight);
  }, [shown, state.session, rung, expanded, state.text, recovery, state.error, editor.error, transcriptLimit]);
  // Keep the live surface compact for ordinary speech, while retaining the
  // complete transcript in the textarea. Long speech and silence diagnostics
  // get the full width; a later pause never collapses the card.
  const contentWidth = compactSpeaking ? 248 : 408;
  const width = rung === 'idle' ? 14 : rung === 'armed' ? 176 : contentWidth + 32;
  const extra = recovery ? 12 + outcomeHeight + 42 : 0;
  if (state.deaf) warningSeen.current = true;
  const warningExtra = warningSeen.current ? 50 : 0;
  const cardHeight = rung === 'idle' ? 14 : rung === 'armed' ? 32 : 48 + height + extra + warningExtra;
  useLayoutEffect(() => {
    pendingSize.current = { width: width + 24, height: cardHeight + 24, position: state.position };
    async function resize() {
      if (resizing.current) return;
      resizing.current = true;
      try {
        while (pendingSize.current) {
          const next = pendingSize.current; pendingSize.current = null;
          await invoke('resize_overlay', next);
        }
      } catch { setConnectionError('Could not resize the overlay. Use the shortcut to stop.'); }
      finally { resizing.current = false; }
    }
    void resize();
  }, [width, cardHeight, state.position]);
  function expand() { maximum.current = expanded ? 48 : 120; setExpanded(!expanded); }
  function follow() { followRef.current = true; setFollowing(true); if (transcript.current) transcript.current.scrollTop = transcript.current.scrollHeight; }
  return <div className="overlay-host" data-position={state.position}>
    <section className="overlay-card" data-rung={rung} data-phase={state.phase} data-tone={success ? 'good' : recovery ? 'bad' : 'live'}
      style={{ width, height: cardHeight }} aria-label="Logia dictation">
      <span className="overlay-pip" aria-label={idle ? 'Idle — microphone off' : label} />
      {/* Only a deliberate recovery interaction may take keyboard focus. */}
      <textarea className={`overlay-transcript ${state.complete ? '' : 'provisional'}`} ref={transcript}
        aria-label={active ? 'Live transcript' : editor.editing ? 'Transcript, editable' : 'Transcript, click to edit'}
        hidden={rung !== 'speaking'} readOnly={!editor.editing} spellCheck={editor.editing} value={shown}
        style={{ width: contentWidth }}
        onClick={() => void editor.begin()}
        onKeyDown={event => { if (event.key === 'Enter' && !editor.editing) { event.preventDefault(); void editor.begin(); } }}
        onChange={event => action('edit', event.target.value)}
        onScroll={event => {
          const node = event.currentTarget;
          followRef.current = node.scrollHeight - node.scrollTop - node.clientHeight < 5;
          setFollowing(followRef.current);
        }} />
      {recovery && <p ref={outcome} className="overlay-outcome" role={editor.error ? 'alert' : undefined}>{editor.error || state.error || (state.text ? deliveryNote(state.delivery, state.complete, state.copied) : 'No speech was captured. Try again with the shortcut.')}</p>}
      <div className="overlay-row">
        {/* claude 2026-09-11: measured peaks from the capture thread, inline in
            the status row. Flat bars while recording mean no audio is arriving,
            which is exactly what a denied microphone looks like on macOS. */}
        {active && <span className="overlay-meter" aria-hidden="true">
          {[0, 1, 2, 3, 4, 5, 6].map(bar => {
            const reach = Math.min(1, Math.pow(state.level, 0.6) * 2.2);
            const weight = 1 - Math.abs(bar - 3) / 4.5;
            return <i key={bar} style={{ height: `${Math.max(14, reach * weight * 100)}%` }} />;
          })}
        </span>}
        <span role="status" className={state.deaf ? 'overlay-deaf' : undefined}>{state.deaf
          ? 'No input'
          : rung === 'armed' && state.phase === 'recording' ? (state.delivery === 'armed' ? 'Listening' : 'Copy only') : label}</span>
        <span className="overlay-clock">{state.phase !== 'loading' && state.phase !== 'warming' ? `${Math.floor(state.seconds / 60)}:${String(state.seconds % 60).padStart(2, '0')}` : ''}</span>
        <span className="overlay-spacer" />
        {!following && rung === 'speaking' && <button className="overlay-follow" aria-label="Follow latest words" title="Follow latest words" onClick={follow}>↓</button>}
        {rung === 'speaking' && <button aria-label={expanded ? 'Compact transcript' : 'Expand transcript'} title={expanded ? 'Compact transcript' : 'Expand transcript'} aria-expanded={expanded} onClick={expand}>{expanded ? '↙' : '↗'}</button>}
        {(state.phase === 'recording' && rung === 'speaking' || state.phase === 'finishing') && <button aria-label="Stop recording" title="Stop recording" disabled={state.phase !== 'recording'} onClick={() => action('stop')}><span className="overlay-stop" /></button>}
        {(active || state.phase === 'warming') && <button aria-label="Cancel" title="Cancel" disabled={state.delivery === 'sending' || state.phase === 'canceling' && !state.error} onClick={() => action('cancel')}>×</button>}
      </div>
      {state.deaf && <p className="overlay-warning" style={{ width: contentWidth }}>No input device detected. Check Microphone access or unmute your microphone.</p>}
      {/* claude 2026-09-11: Open editor and Dismiss are gone. Editing happens in
          the transcript above, and the shortcut ends the session — a separate
          window and a separate dismiss button were both steps for nothing. */}
      {recovery && <div className="overlay-recovery">
        <button className="overlay-copy" disabled={!state.text} onClick={() => action('copy')}>{state.copied ? 'Copied' : 'Copy again'}</button>
        {state.delivery === 'permission' && <button className="overlay-permission" onClick={() => action('permissions')}>Accessibility settings</button>}
        <span className="overlay-spacer" />
        <span className="overlay-hint">{state.shortcut} to dictate again</span>
        <button className="overlay-close" aria-label="Close recovery" title="Close" onClick={() => action('dismiss')}>×</button>
      </div>}
    </section>
    {connectionError && <p className="overlay-connection-error" role="alert">{connectionError}</p>}
  </div>;
}
