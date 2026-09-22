import { useEffect, useState, type ReactNode } from 'react';
import { Vocabulary } from './vocabulary';
import { ModelsPage } from './models-page';
import type { DictionaryRule } from './dictionary';
import { Brand } from './brand';
import { PermissionSetup } from './permission-setup';
import type { Permissions } from './use-permissions';
import type { Preferences } from './preferences';
import './settings.css';

export type SettingsPage = 'general' | 'models' | 'vocabulary' | 'advanced' | 'test' | 'recovery';
type Props = {
  phase: string; shortcut: string; shortcutError: string; retryable: boolean;
  error: string; progress: number; native: boolean; preferences: Preferences;
  onPreferences: (next: Preferences) => void; onShortcut: (shortcut: string) => Promise<void>;
  onRetryShortcut: () => void; onDownload: () => void; onError: (message: string) => void;
  onPrepare: () => void; onTest: () => void; page: SettingsPage;
  onPage: (page: SettingsPage) => void; children?: ReactNode;
  rules: DictionaryRule[]; onRules: (rules: DictionaryRule[]) => void;
  permissions: Permissions;
};
const pages: Record<SettingsPage, { title: string; detail: string; icon: string }> = {
  general: { title: 'General', detail: 'Make dictation fit the way you work.', icon: 'M4 7h16M4 17h16M9 4v6M15 14v6' },
  models: { title: 'Models', detail: 'Your voice, transcribed on your device.', icon: 'm12 3 8 5v8l-8 5-8-5V8l8-5Zm0 9v9M4 8l8 4 8-4' },
  vocabulary: { title: 'Vocabulary', detail: 'Fix the names and terms recognition gets wrong.', icon: 'M4 5h16M4 12h10M4 19h7M17 15l3 3-3 3M20 18h-6' },
  advanced: { title: 'Advanced', detail: 'Privacy and tools for checking your setup.', icon: 'M4 6h10m4 0h2M4 12h2m4 0h10M4 18h10m4 0h2M14 3v6M6 9v6m8 0v6' },
  test: { title: 'Voice test', detail: 'Check your microphone and local recognition here.', icon: 'M4 10v4m4-8v12m4-15v18m4-15v12m4-8v4' },
  recovery: { title: 'Recovery', detail: 'Review and copy the text from your current session.', icon: 'M4 5h16v14H4V5Zm0 9h5l2 3h2l2-3h5' },
};
const presets = ['Control+Alt+Space', 'Control+Shift+Space', 'Alt+Shift+Space'];
const readiness: Record<string, [string, string]> = {
  checking: ['Checking your model', 'Looking for the local voice model. Your microphone is off.'],
  setup: ['Set up your voice model', 'Download the English model to start using local dictation.'],
  downloading: ['Downloading your voice model', 'Keep Logia open while the model downloads. Your microphone is off.'],
  warming: ['Preparing your voice model', 'Loading local recognition. Dictation will be available shortly; your microphone is off.'],
  loading: ['Preparing to record', 'Your dictation session is starting.'],
  recording: ['Recording in progress', 'Use your global shortcut or the overlay to stop recording.'],
  finishing: ['Finishing your transcript', 'Recognition is completing the last words.'],
  canceling: ['Stopping recording', 'Waiting for the current session to stop before another can begin.'],
  ready: ['Local model available', 'Use your global shortcut from the app where you want to dictate.'],
};
function shortcutLabel(value: string) { return value.replaceAll('+', ' + '); }
function Icon({ page }: { page: SettingsPage }) {
  return <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d={pages[page].icon} /></svg>;
}
function Row({ title, detail, children }: { title: string; detail: string; children: ReactNode }) {
  return <div className="settings-row"><div className="settings-row-copy"><h3>{title}</h3><p>{detail}</p></div><div className="settings-row-control">{children}</div></div>;
}
function Section({ title, children }: { title: string; children: ReactNode }) {
  return <section className="settings-group"><h2>{title}</h2><div className="settings-rows">{children}</div></section>;
}

export function Settings(props: Props) {
  const { page, onPage, preferences, onPreferences, phase, native } = props;
  const [draft, setDraft] = useState(preferences.shortcut);
  const [applying, setApplying] = useState(false);
  useEffect(() => { setDraft(preferences.shortcut); }, [preferences.shortcut]);
  const busy = ['checking', 'downloading', 'warming', 'loading', 'recording', 'finishing', 'canceling'].includes(phase);
  const modelMissing = ['setup', 'checking', 'downloading'].includes(phase);
  const modelStatus = phase === 'setup' ? 'Not downloaded' : phase === 'checking' ? 'Checking…' : phase === 'downloading' ? 'Downloading…' : phase === 'warming' ? 'Preparing…' : 'Installed';
  function openTest() { onPage('test'); props.onTest(); }
  async function applyShortcut() {
    setApplying(true);
    try { await props.onShortcut(draft); } catch { /* The controller exposes the registration error. */ }
    finally { setApplying(false); }
  }
  return <div className="settings-shell">
    <aside className="settings-sidebar">
      <Brand />
      <nav aria-label="Settings">
        {(['general', 'models', 'vocabulary', 'advanced'] as SettingsPage[]).map(item => <button key={item} className="settings-nav-item" aria-current={page === item ? 'page' : undefined} onClick={() => onPage(item)}><Icon page={item} />{pages[item].title}</button>)}
      </nav>
      <div className="settings-sidebar-tools">
        <span className="settings-nav-label">Tools</span>
        <button className="settings-nav-item" aria-current={page === 'test' ? 'page' : undefined} onClick={openTest}><Icon page="test" />Voice test</button>
        <button className="settings-nav-item" aria-current={page === 'recovery' ? 'page' : undefined} onClick={() => onPage('recovery')}><Icon page="recovery" />Recovery</button>
      </div>
      <div className="settings-sidebar-foot"><span className="settings-local-dot" />Local dictation<span className="settings-version">Personal preview</span></div>
    </aside>
    <main className="settings-content">
      {props.permissions.setup ? <PermissionSetup permissions={props.permissions} /> : <>
      <header className="settings-heading"><h1>{pages[page].title}</h1><p>{pages[page].detail}</p></header>
      {!native && <p className="settings-notice">Browser preview. Open the desktop app to use dictation and global shortcuts.</p>}
      {page === 'general' && <>
        {native && <section className="settings-permissions" aria-labelledby="permissions-title">
          <div><h2 id="permissions-title">Permissions</h2><p>Microphone: {props.permissions.microphone === 'authorized' ? 'allowed' : 'needs attention'} · Typing access: {props.permissions.accessibility === 'authorized' ? 'allowed' : 'needs attention'}</p></div>
          <button className="settings-button" onClick={props.permissions.review}>Review permissions</button>
        </section>}
        {native && <div className="settings-readiness">
          <div role="status"><h2>{(readiness[phase] ?? readiness.checking)[0]}{phase === 'downloading' ? ` · ${props.progress}%` : ''}</h2><p>{(readiness[phase] ?? readiness.checking)[1]}</p></div>
          {(modelMissing || phase === 'warming') && <button className="settings-button" onClick={() => onPage('models')}>{phase === 'setup' ? 'Set up model' : 'View model'}</button>}
        </div>}
        <Section title="Dictation">
          <Row title="Global shortcut" detail="Press once to record in any app. Press again to stop.">
            <div className="settings-shortcut-picker"><select aria-label="Global shortcut" value={draft} disabled={!native || busy || applying} onChange={event => setDraft(event.target.value)}>
              {!presets.includes(draft) && <option value={draft}>{shortcutLabel(draft)}</option>}
              {presets.map(value => <option key={value} value={value}>{shortcutLabel(value)}</option>)}
            </select><button className="settings-button" disabled={!native || busy || applying || draft === preferences.shortcut} onClick={() => void applyShortcut()}>{applying ? 'Applying…' : 'Apply'}</button></div>
          </Row>
          {props.shortcutError ? <div className="settings-inline-error" role="alert">{props.shortcutError}{props.retryable && <button className="settings-button" disabled={busy || applying} onClick={props.onRetryShortcut}>Retry shortcut</button>}</div> : native && props.shortcut && <p className="settings-shortcut-status">Active shortcut: <kbd>{props.shortcut}</kbd></p>}

        </Section>
        <Section title="Recording overlay">
          <Row title="Position" detail="Keep captions near the top or bottom of your screen."><select aria-label="Overlay position" value={preferences.position} onChange={event => onPreferences({ ...preferences, position: event.target.value as Preferences['position'] })}><option value="bottom">Bottom</option><option value="top">Top</option></select></Row>
          <Row title="Show idle indicator" detail="A small amber dot shows where the overlay will appear."><button type="button" className="settings-switch" role="switch" aria-label="Show idle indicator" aria-checked={preferences.showIdle} onClick={() => onPreferences({ ...preferences, showIdle: !preferences.showIdle })}><span /></button></Row>
        </Section>
        <Section title="Appearance"><Row title="Theme" detail="Choose the appearance of the settings window."><select aria-label="Theme" value={preferences.theme} onChange={event => onPreferences({ ...preferences, theme: event.target.value as Preferences['theme'] })}><option value="light">Light</option><option value="dark">Dark</option></select></Row></Section>
      </>}
      {page === 'models' && <ModelsPage native={native} phase={phase} progress={props.progress}
        busy={busy} onPrepare={props.onPrepare} onError={props.onError} />}
      {page === 'vocabulary' && <Vocabulary rules={props.rules} onRules={props.onRules} native={native} />}
      {page === 'advanced' && <>
        <Section title="Privacy">
          <Row title="On-device recognition" detail="Your audio is transcribed locally. No account or cloud service is needed."><span className="settings-value">Local</span></Row>
          <Row title="Recording history" detail="Logia does not save audio or transcript history. Recovery holds only the current session."><span className="settings-value">Off</span></Row>
          <Row title="Text processing" detail="Recognition can revise provisional words when finishing the transcript. No additional rewriting pass is applied."><span className="settings-value">No rewriting</span></Row>
        </Section>
        <Section title="Tools">
          <Row title="Voice test" detail="Check recognition in a dedicated space before dictating into another app."><button className="settings-button" onClick={openTest}>Open voice test</button></Row>
          <Row title="Recover text" detail="Review or copy text retained from your current session."><button className="settings-button" onClick={() => onPage('recovery')}>Open recovery</button></Row>
        </Section>
      </>}
      {(page === 'test' || page === 'recovery') && <div className="settings-tool-content">{props.children}</div>}
      {props.error && <div className="settings-inline-error" role="alert">{props.error}</div>}

      </>}
    </main>
  </div>;
}
