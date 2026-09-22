import { useEffect, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { Settings, type SettingsPage } from './settings';
import { usePermissions } from './use-permissions';
import { Overlay } from './overlay';
import { readPreferences, savePreferences, type Preferences } from './preferences';
import { readDictionary, saveDictionary, type DictionaryRule } from './dictionary';
import { useDictation } from './use-dictation';
import { useDictationWindow } from './use-dictation-window';
import { useOverlayBridge, type OverlayGate } from './use-overlay-bridge';
import { SessionWorkspace } from './session-workspace';
import type { OverlayAction } from './dictation-types';
import '@fontsource/archivo/400.css';
import '@fontsource/archivo/500.css';
import '@fontsource/newsreader/400.css';
import '@fontsource/newsreader/400-italic.css';
import './style.css';

function App() {
  const native = isTauri();
  const [preferences, setPreferences] = useState(readPreferences);
  const [page, setPage] = useState<SettingsPage>('general');
  const [rules, setRules] = useState<DictionaryRule[]>(readDictionary);
  const permissions = usePermissions(native);
  const shortcutChecking = useRef(false);
  // claude 2026-09-11: Rust owns matching, so it must hold the current rules.
  function updateRules(next: DictionaryRule[]) {
    saveDictionary(next); setRules(next);
    if (native) void invoke('set_dictionary', { rules: next }).catch(e => session.setError(String(e)));
  }
  // claude 2026-09-11: the bridge needs session state, and the session needs
  // the bridge's readiness gate. A ref breaks the cycle; it is only read inside
  // async handlers, never during render.
  const overlayGate = useRef<OverlayGate | null>(null);
  const session = useDictation(native, overlayGate);
  const windowView = useDictationWindow(native, preferences.shortcut, () => {
    // Permission loss must never take away the shortcut used to stop capture.
    if (session.phase === 'recording') { void session.stop(); return; }
    if (session.phase !== 'ready' || shortcutChecking.current) return;
    shortcutChecking.current = true;
    void (async () => {
      try {
        const status = await permissions.refresh();
        // A permission request already in progress owns this transition.
        if (!status) return;
        if (status.microphone !== 'authorized' || status.accessibility !== 'authorized') {
          permissions.review(); await invoke('show_main_window'); return;
        }
        await session.toggleFromShortcut();
      } catch (e) { session.setError(String(e)); }
      finally { shortcutChecking.current = false; }
    })();
  }, () => void session.dismissWindow());
  function updatePreferences(next: Preferences) { savePreferences(next); setPreferences(next); }
  async function changeShortcut(shortcut: string) {
    await windowView.register(shortcut);
    // claude 2026-09-11: registration can outlast other edits. Merge into the
    // latest preferences rather than the snapshot captured when Apply was hit.
    setPreferences(latest => { const next = { ...latest, shortcut }; savePreferences(next); return next; });
  }
  function overlayAction(action: OverlayAction, text?: string) {
    if (action === 'stop') void session.stop();
    if (action === 'cancel') void session.cancel();
    if (action === 'copy') void session.copy();
    if (action === 'dismiss') session.dismissOverlay();
    if (action === 'permissions') { permissions.review(); void invoke('show_main_window').catch(e => session.setError(String(e))); }
    // claude 2026-09-11: edits now arrive from the overlay's own transcript
    // rather than opening a second window.
    if (action === 'edit' && typeof text === 'string' && !session.active) session.edit(text);
  }
  overlayGate.current = useOverlayBridge(native, {
    session: session.session, phase: session.phase, text: session.text, complete: session.complete,
    error: session.error, seconds: session.seconds, delivery: session.delivery.status, copied: session.copied,
    dismissed: session.dismissed, shortcut: windowView.shortcut, position: preferences.position, showIdle: preferences.showIdle,
    level: session.level, deaf: session.deaf
  }, overlayAction, session.setError, () => setPage('recovery'));
  useEffect(() => { document.documentElement.dataset.theme = preferences.theme; }, [preferences.theme]);
  useEffect(() => { if (native) void invoke('set_dictionary', { rules }).catch(() => {}); }, [native]);
  useEffect(() => {
    if (native && session.phase === 'setup') {
      setPage('models'); void invoke('show_main_window').catch(e => session.setError(String(e)));
    }
  }, [native, session.phase]);
  useEffect(() => {
    if (permissions.setup && !session.active) void invoke('show_main_window').catch(() => {});
  }, [permissions.setup]);
  useEffect(() => {
    const key = (event: KeyboardEvent) => {
      if (!event.repeat && (event.metaKey || event.ctrlKey) && event.key === 'Enter') {
        if (session.phase === 'recording') { event.preventDefault(); void session.stop(); }
        else if (page === 'test' && session.phase === 'ready') { event.preventDefault(); void session.start(); }
      }
    };
    window.addEventListener('keydown', key); return () => window.removeEventListener('keydown', key);
  }, [page, session.phase]);
  return <Settings phase={session.phase} shortcut={windowView.shortcut} shortcutError={windowView.shortcutError}
    retryable={windowView.retryable} error={session.error} progress={session.progress} native={native}
    preferences={preferences} onPreferences={updatePreferences} onShortcut={changeShortcut}
    onRetryShortcut={() => void windowView.register().catch(() => {})}
    permissions={permissions}
    onDownload={() => void session.download()} onPrepare={() => void session.prepare()} onTest={() => {}}
    onError={session.setError}
    page={page} onPage={setPage} rules={rules} onRules={updateRules}><SessionWorkspace session={session} testing={page === 'test'} /></Settings>;
}
const overlay = new URLSearchParams(location.search).has('overlay');
document.documentElement.dataset.surface = overlay ? 'overlay' : 'settings';
createRoot(document.getElementById('root')!).render(overlay ? <Overlay /> : <App />);
