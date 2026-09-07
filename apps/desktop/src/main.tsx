import React, { useEffect, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import '@fontsource/archivo/400.css';
import '@fontsource/archivo/500.css';
import '@fontsource/newsreader/400.css';
import '@fontsource/newsreader/400-italic.css';
import './style.css';

type Phase = 'checking' | 'setup' | 'downloading' | 'ready' | 'loading' | 'recording' | 'finishing' | 'canceling';
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
  const captionElement = useRef<HTMLSpanElement>(null);
  const phaseRef = useRef(phase);
  phaseRef.current = phase;
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
        if (event.type === 'loading') setPhase('loading');
        if (event.type === 'listening') { setPhase('recording'); setSeconds(0); }
        if (event.type === 'partial') setText(event.text ?? '');
        if (event.type === 'final') { setText(event.text ?? ''); setComplete(true); setPhase('finishing'); }
        if (event.type === 'error') setError(event.message ?? 'Recognition stopped. Try again.');
        if (event.type === 'stopped') setPhase('ready');
      });
      const download = await listen<number>('model-progress', ({ payload }) => setProgress(payload));
      if (disposed) { updates(); download(); return; }
      unsubscribers.push(updates, download);
      try { setPhase(await invoke<boolean>('model_ready') ? 'ready' : 'setup'); }
      catch (e) { setError(String(e)); setPhase('setup'); }
    };
    void register().catch(() => { if (!disposed) { setError('Could not connect to recognition. Close and reopen Logia.'); setPhase('setup'); } });
    return () => { disposed = true; unsubscribers.forEach(unsubscribe => unsubscribe()); };
  }, [native]);

  useEffect(() => {
    if (phase !== 'recording') return;
    const timer = setInterval(() => setSeconds(value => value + 1), 1000);
    return () => clearInterval(timer);
  }, [phase]);
  useEffect(() => { if (phase === 'recording' && seconds >= 60) void stop(); }, [seconds, phase]);
  useEffect(() => { document.documentElement.dataset.theme = dark ? 'dark' : 'light'; }, [dark]);
  useEffect(() => { if (captionElement.current) captionElement.current.scrollTop = captionElement.current.scrollHeight; }, [text]);
  useEffect(() => {
    const key = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') {
        event.preventDefault();
        if (phase === 'ready') void start();
        if (phase === 'recording') void stop();
      }
    };
    window.addEventListener('keydown', key);
    return () => window.removeEventListener('keydown', key);
  }, [phase]);

  async function start() {
    if (!native || phase !== 'ready') return;
    setError(''); setText(''); setComplete(false); setCopied(false); setPhase('loading');
    try { generation.current = Math.max(generation.current, await invoke<number>('start_recording')); }
    catch (e) { setError(String(e)); setPhase('ready'); }
  }
  async function stop() {
    setPhase('finishing');
    try { await invoke('stop_recording'); }
    catch (e) { setError(String(e)); }
  }
  async function cancel() {
    phaseRef.current = 'canceling';
    setPhase('canceling'); setText(''); setComplete(false);
    try { await invoke('cancel_recording'); }
    catch (e) { setError(String(e)); }
  }
  async function download() {
    setPhase('downloading'); setError(''); setProgress(0);
    try { await invoke('download_model'); setPhase('ready'); }
    catch (e) { setError(String(e)); setPhase('setup'); }
  }
  async function copy() {
    try { await invoke('plugin:clipboard-manager|write_text', { text }); setCopied(true); }
    catch { setError('Could not copy. Select the text and use your usual copy shortcut.'); }
  }
  const caption = text || (phase === 'recording' ? 'Listening for your words…' : '');
  const status = phase === 'recording' ? 'Listening' : phase === 'loading' ? 'Preparing' : phase === 'finishing' ? 'Finishing' : phase === 'canceling' ? 'Stopping' : complete && text ? 'Ready to use' : 'Ready when you are';
  const setup = ['setup','downloading','checking'].includes(phase);

  return <main>
    <header><div className="wordmark"><span className="brand-pip"/>Logia</div><div className="header-actions"><span>Personal preview</span><button className="quiet" onClick={() => setDark(!dark)}>{dark ? 'Light' : 'Dark'}</button></div></header>
    <section className="intro"><h1>Room for your words.</h1><p>Speak naturally. Keep the thought moving.</p></section>
    {setup ? <section className="setup" aria-labelledby="setup-title">
      <span className="setup-mark" aria-hidden="true">a</span>
      <div><h2 id="setup-title">A voice of your own.</h2><p>Download the English model once. After that, your speech is transcribed on this device.</p>
      {native ? <button className="primary" disabled={phase !== 'setup'} onClick={() => void download()}>{phase === 'checking' ? 'Checking model…' : phase === 'downloading' ? `Downloading · ${progress}%` : 'Download English model'}</button> : <p className="browser-note">Open the Logia desktop app to record. This browser view cannot access the recognizer.</p>}
      <small>{phase === 'downloading' ? 'You can leave this window open while it downloads.' : '199 MB · no account needed'}</small></div>
    </section> : <section className="workspace" aria-label="Dictation">
      <div className="workspace-heading"><span>{complete && text ? 'Your words' : 'A little space to think'}</span><span className="status-label">{status}</span></div>
      <textarea aria-label="Transcript" spellCheck value={text} readOnly={active} onChange={event => { setText(event.target.value); setCopied(false); }} placeholder="A first thought. A message. A bit of code. Start recording and let the words come." />
      <div className={`breathing ${phase === 'recording' && text ? 'speaking' : active ? 'armed' : 'rest'}`} aria-label={status}>
        <span className={`pip ${active ? 'live' : ''}`}/>
        {active && <>{phase === 'recording' && text ? <span className="caption" ref={captionElement}>{caption}</span> : <span className="overlay-label">{status}</span>}<span className="clock">{phase === 'recording' ? `0:${String(seconds).padStart(2,'0')}` : ''}</span></>}
      </div>
      <div className="controls"><div><button className="quiet" disabled={!text || active} onClick={() => { setText(''); setComplete(false); }}>Clear</button><button className="copy" disabled={!text || active} onClick={() => void copy()}>{copied ? 'Copied' : complete ? 'Copy text' : 'Copy partial'}</button></div>
      <div className="record-actions">{active && <button className="quiet" onClick={() => void cancel()} disabled={phase === 'canceling'}>Cancel</button>}
      <button className={`primary ${phase === 'recording' ? 'recording' : ''}`} disabled={!['ready','recording'].includes(phase)} onClick={() => void (phase === 'recording' ? stop() : start())}><span className="record-icon"/>{phase === 'recording' ? 'Stop recording' : active ? `${status}…` : 'Start recording'}</button></div></div>
    </section>}
    {error && <div className="error" role="alert">{error}</div>}
    <footer><p><span className="privacy-dot"/>On-device. No transcript history.</p><p>English preview · up to 60 seconds</p></footer>
    <p className="footnote">{setup ? 'Your microphone starts only when you choose Record.' : '⌘ Enter to record or stop in this window. Copy your text wherever you need it.'}</p>
  </main>;
}
createRoot(document.getElementById('root')!).render(<App/>);
