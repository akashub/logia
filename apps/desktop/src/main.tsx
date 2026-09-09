import React, { useEffect, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { TranscriptEditor } from './transcript-editor';
import { useDictationWindow } from './use-dictation-window';
import '@fontsource/archivo/400.css';
import '@fontsource/archivo/500.css';
import '@fontsource/newsreader/400.css';
import '@fontsource/newsreader/400-italic.css';
import './style.css';
import './floating.css';

type Phase = 'checking' | 'setup' | 'downloading' | 'warming' | 'ready' | 'loading' | 'recording' | 'finishing' | 'canceling';
type Update = { generation: number; event: { type: string; text?: string; message?: string } };
function App() {
  const native = isTauri();
  const [phase, setPhase] = useState<Phase>(native ? 'checking' : 'setup');
  const [text, setText] = useState('');
  const [error, setError] = useState('');
  const [progress, setProgress] = useState(0);
  const [seconds, setSeconds] = useState(0);
  const [copied, setCopied] = useState(false);
  const [complete, setComplete] = useState(false);
  const [dark, setDark] = useState(false);
  const generation = useRef(0);
  const controlIntent = useRef(0);
  const phaseRef = useRef(phase);
  phaseRef.current = phase;
  const windowView = useDictationWindow(native, () => void toggleFromShortcut());
  function transition(next: Phase) { phaseRef.current = next; setPhase(next); }
  const active = ['loading','recording','finishing','canceling'].includes(phase);

  useEffect(() => {
    if (!native) return;
    let disposed = false;
    const unsubscribers: (() => void)[] = [];
    const register = async () => {
      const updates = await listen<Update>('recognition', ({ payload }) => {
        if (disposed || payload.generation < generation.current) return;
        generation.current = payload.generation;
        const event = payload.event;
        if (phaseRef.current === 'canceling' && event.type !== 'stopped') return;
        if (event.type === 'loading' && phaseRef.current !== 'warming') transition('loading');
        if (event.type === 'listening') { transition('recording'); setSeconds(0); }
        // Empty interim snapshots do not retract the visible paragraph.
        // An authoritative final (including an empty final) still replaces it.
        if (event.type === 'partial' && event.text?.trim()) setText(event.text);
        if (event.type === 'final') { setText(event.text ?? ''); setComplete(true); transition('finishing'); }
        if (event.type === 'error') setError(event.message ?? 'Recognition stopped. Try again.');
        if (event.type === 'stopped') transition('ready');
      });
      const download = await listen<number>('model-progress', ({ payload }) => setProgress(payload));
      if (disposed) { updates(); download(); return; }
      unsubscribers.push(updates, download);
      try { if (await invoke<boolean>('model_ready')) await prepare(); else transition('setup'); }
      catch (e) { setError(String(e)); transition('setup'); }
    };
    void register().catch(() => { if (!disposed) { setError('Could not connect to recognition. Close and reopen Logia.'); transition('setup'); } });
    return () => { disposed = true; unsubscribers.forEach(unsubscribe => unsubscribe()); };
  }, [native]);

  useEffect(() => {
    if (phase !== 'recording') return;
    const timer = setInterval(() => setSeconds(value => value + 1), 1000);
    return () => clearInterval(timer);
  }, [phase]);
  useEffect(() => { if (phase === 'recording' && seconds >= 60) void stop(); }, [seconds, phase]);
  useEffect(() => { document.documentElement.dataset.theme = dark ? 'dark' : 'light'; }, [dark]);
  useEffect(() => {
    const key = (event: KeyboardEvent) => {
      if (!event.repeat && (event.metaKey || event.ctrlKey) && event.key === 'Enter') {
        event.preventDefault();
        if (phase === 'ready') void start();
        if (phase === 'recording') void stop();
      }
    };
    window.addEventListener('keydown', key);
    return () => window.removeEventListener('keydown', key);
  }, [phase]);

  async function toggleFromShortcut() {
    if (phaseRef.current === 'recording') { await stop(); return; }
    if (phaseRef.current !== 'ready') return;
    const intent = controlIntent.current;
    try { if (await windowView.change(true, true) && controlIntent.current === intent) await start(); }
    catch (e) { setError(String(e)); }
  }
  async function changeWindow() {
    controlIntent.current++;
    try { await windowView.change(!windowView.floating); }
    catch (e) { setError(String(e)); }
  }

  async function start() {
    if (!native || phaseRef.current !== 'ready') return;
    controlIntent.current++;
    setError(''); setText(''); setComplete(false); setCopied(false); transition('loading');
    try { generation.current = Math.max(generation.current, await invoke<number>('start_recording')); }
    catch (e) { setError(String(e)); transition('ready'); }
  }
  async function stop() {
    if (phaseRef.current !== 'recording') return;
    controlIntent.current++;
    transition('finishing');
    try { await invoke('stop_recording'); }
    catch (e) { setError(String(e)); }
  }
  async function cancel() {
    controlIntent.current++;
    const preparing = phaseRef.current === 'warming';
    phaseRef.current = 'canceling';
    transition('canceling');
    if (!preparing) { setText(''); setComplete(false); }
    try { await invoke('cancel_recording'); }
    catch (e) { setError(String(e)); }
  }
  async function download() {
    transition('downloading'); setError(''); setProgress(0);
    try { await invoke('download_model'); await prepare(); }
    catch (e) { setError(String(e)); transition('setup'); }
  }
  async function prepare() {
    phaseRef.current = 'warming';
    transition('warming');
    try { generation.current = Math.max(generation.current, await invoke<number>('warmup_recognizer')); }
    catch (e) { setError(String(e)); transition('ready'); }
  }
  async function copy() {
    try { await invoke('plugin:clipboard-manager|write_text', { text }); setCopied(true); }
    catch { setError('Could not copy. Select the text and use your usual copy shortcut.'); }
  }
  const status = phase === 'recording' ? 'Listening' : phase === 'warming' ? 'Preparing voice model' : phase === 'loading' ? 'Preparing' : phase === 'finishing' ? 'Finishing' : phase === 'canceling' ? 'Stopping' : complete && text ? 'Ready to use' : 'Ready when you are';
  const setup = ['setup','downloading','checking'].includes(phase);

  return <main data-view={windowView.floating ? 'floating' : 'full'}>
    <header><div className="wordmark"><span className="brand-pip"/>Logia</div><div className="header-actions"><span className="preview-label">Personal preview</span><button className="quiet" onClick={() => setDark(!dark)}>{dark ? 'Light' : 'Dark'}</button>{native && <button className="quiet window-toggle" disabled={windowView.changing} onClick={() => void changeWindow()}>{windowView.floating ? 'Expand window' : 'Float window'}</button>}</div></header>
    <section className="intro"><h1>Room for your words.</h1><p>Take your time. There’s room for the whole thought.</p></section>
    {setup ? <section className="setup" aria-labelledby="setup-title">
      <span className="setup-mark" aria-hidden="true">a</span>
      <div><h2 id="setup-title">A voice of your own.</h2><p>Download the English model once. After that, your speech is transcribed on this device.</p>
      {native ? <button className="primary" disabled={phase !== 'setup'} onClick={() => void download()}>{phase === 'checking' ? 'Checking model…' : phase === 'downloading' ? `Downloading · ${progress}%` : 'Download English model'}</button> : <p className="browser-note">Open the Logia desktop app to record. This browser view cannot access the recognizer.</p>}
      <small>{phase === 'downloading' ? 'You can leave this window open while it downloads.' : '199 MB · no account needed'}</small></div>
    </section> : <section className="workspace" aria-label="Dictation">
      <div className="workspace-heading"><span>Your words</span><span className={`status-label ${active ? 'is-active' : ''}`} role="status"><span className="status-dot"/>{status}<span className="clock">{active ? `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2,'0')}` : ''}</span></span></div>
      <TranscriptEditor text={text} active={active} streaming={(phase === 'recording' || phase === 'finishing') && !complete} onChange={value => { setText(value); setCopied(false); }} />
      <div className="session-note">{phase === 'warming' ? 'Preparing local recognition. Your microphone is off.' : phase === 'recording' ? 'Pauses are welcome. Recording continues until you choose Stop or reach 60 seconds.' : phase === 'finishing' ? 'Finishing the last words. Your paragraph stays here.' : complete && text ? 'Ready to edit and copy. Your paragraph stays until you clear it or start again.' : 'Your whole paragraph appears here, with room to pause and think.'}</div>
      <div className="controls"><div><button className="quiet" disabled={!text || active} onClick={() => { setText(''); setComplete(false); }}>Clear</button><button className="copy" disabled={!text || active} onClick={() => void copy()}>{copied ? 'Copied' : complete ? 'Copy text' : 'Copy partial'}</button></div>
      <div className="record-actions">{(active || phase === 'warming') && <button className="quiet" onClick={() => void cancel()} disabled={phase === 'canceling'}>Cancel</button>}
      <button className={`primary ${phase === 'recording' ? 'recording' : ''}`} disabled={!['ready','recording'].includes(phase)} onClick={() => void (phase === 'recording' ? stop() : start())}><span className="record-icon"/>{phase === 'recording' ? 'Stop recording' : active || phase === 'warming' ? `${status}…` : 'Start recording'}</button></div></div>
    </section>}
    {error && <div className="error" role="alert">{error}</div>}
    <footer><p><span className="privacy-dot"/>On-device. No transcript history.</p><p>English preview · up to 60 seconds</p></footer>
    {native && <div className="shortcut-note">{windowView.shortcut && <><kbd>{windowView.shortcut}</kbd><span>Record / stop from any app</span></>}{windowView.shortcutError && <><span role="alert">{windowView.shortcutError}</span><button className="quiet" onClick={() => void windowView.register()}>Retry shortcut</button></>}</div>}
    <p className="footnote">{setup ? 'Your microphone starts only when you choose Record.' : 'Copy your words wherever you need them.'}</p>
  </main>;
}
createRoot(document.getElementById('root')!).render(<App/>);
