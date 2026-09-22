import type { Permissions } from './use-permissions';

export function PermissionSetup({ permissions: p }: { permissions: Permissions }) {
  const mic = p.microphone === 'authorized', ready = mic && p.accessibility === 'authorized';
  const unknown = (!mic && p.microphone === 'unknown') || (mic && p.accessibility === 'unknown');
  const label = p.busy ? 'Checking access…' : ready ? 'Finish setup' : unknown ? 'Check permissions'
    : mic ? 'Enable typing access' : p.microphone === 'denied' ? 'Open Microphone settings'
    : p.microphone === 'restricted' ? 'Check permissions' : 'Allow microphone';
  const click = () => {
    if (ready) p.finish();
    else if (unknown || p.microphone === 'restricted') void p.refresh();
    else void p.act(mic ? 'accessibility' : 'microphone');
  };
  return <section className="settings-permission-wizard" aria-labelledby="setup-title">
    <h1 id="setup-title">Set up Logia</h1>
    <p className="setup-lede">Two permissions to dictate where you work.</p>
    <ol className="setup-steps" aria-label="Setup progress">
      <li className={`setup-step ${mic ? 'is-done' : 'is-current'}`} aria-current={!mic ? 'step' : undefined}><b>{mic ? '✓' : '1'}</b><span>Microphone</span></li>
      <li className="setup-step-line" aria-hidden="true" />
      <li className={`setup-step ${ready ? 'is-done' : mic ? 'is-current' : ''}`} aria-current={mic ? 'step' : undefined}><b>{ready ? '✓' : '2'}</b><span>Typing access</span></li>
    </ol>
    <div className="settings-permission-card">
      <div className="setup-card-icon" aria-hidden="true"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round">
        {mic ? <path d="M5 18 19 4M8 4h11v11M5 10v10h10" /> : <><rect x="8" y="2" width="8" height="13" rx="4" /><path d="M5 11a7 7 0 0 0 14 0M12 18v4m-4 0h8" /></>}
      </svg></div>
      <div className="setup-card-copy">
        <h2>{mic ? 'Let Logia type for you' : 'Let Logia hear you'}</h2>
        <p>{unknown ? 'Checking the permission reported by this Mac.' : ready ? 'Microphone and typing access are allowed. You’re ready to dictate.'
          : mic ? 'Enable Logia in Accessibility so your words can go where your text cursor is.'
          : p.microphone === 'denied' ? 'Microphone access is off. Enable Logia in System Settings, then return here.'
          : p.microphone === 'restricted' ? 'Microphone access is restricted by Screen Time or your administrator.'
          : 'Your audio stays on this Mac. Logia listens only when you start dictating.'}</p>
        {mic && !ready && !unknown && <p className="setup-help">If Logia isn’t listed, use + to add Logia from Applications.</p>}
      </div>
      <button className="settings-button settings-button-primary" disabled={p.busy} onClick={click}>{label}</button>
      {p.error && <p className="settings-inline-error setup-error" role="alert">{p.error}</p>}
    </div>
    <p className="setup-footnote">{ready ? 'Use your global shortcut to start and stop.' : 'Access updates automatically when you return to Logia.'}</p>
  </section>;
}
