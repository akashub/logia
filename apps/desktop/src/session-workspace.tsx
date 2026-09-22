import { TranscriptEditor } from './transcript-editor';
import { deliveryNote } from './use-delivery';
import type { useDictation } from './use-dictation';

export function SessionWorkspace({ session: s, testing }: { session: ReturnType<typeof useDictation>; testing: boolean }) {
  const status = s.phase === 'recording' ? 'Listening' : s.phase === 'warming' || s.phase === 'loading' ? 'Preparing'
    : s.phase === 'finishing' ? 'Finishing' : s.phase === 'canceling' ? 'Stopping' : s.complete && s.delivery.status === 'sent' ? 'Text sent' : s.complete && s.delivery.status === 'dispatched' ? 'Paste sent' : 'Ready';
  return <section className="workspace" aria-label={testing ? 'Voice test' : 'Recovered text'}>
    <div className="workspace-heading"><span>{testing ? 'Test transcript' : 'Current session'}</span><span className={`status-label ${s.active ? 'is-active' : ''}`} role="status"><span className="status-dot" />{status}</span></div>
    <TranscriptEditor text={s.text} active={s.active} streaming={(s.phase === 'recording' || s.phase === 'finishing') && !s.complete} onChange={s.edit} />
    <div className="session-note">{s.phase === 'warming' ? 'Preparing recognition. Your microphone is off.' : s.phase === 'recording' ? 'Recording continues through pauses, for up to 60 seconds.' : deliveryNote(s.delivery.status, s.complete, s.copied)}</div>
    <div className="controls"><div>
      <button className="quiet" disabled={!s.text || s.active} onClick={s.clear}>Clear</button>
      <button className="copy" disabled={!s.text || s.active} onClick={() => void s.copy()}>{s.copied ? 'Copied' : s.complete ? 'Copy text' : 'Copy partial'}</button>
    </div><div className="record-actions">
      {(s.active || s.phase === 'warming') && <button className="quiet" onClick={() => void s.cancel()} disabled={s.delivery.status === 'sending' || s.phase === 'canceling' && !s.error}>Cancel</button>}
      {(testing || s.active) && <button className={`primary ${s.phase === 'recording' ? 'recording' : ''}`} disabled={s.isDismissing || !['ready', 'recording'].includes(s.phase)} onClick={() => void (s.phase === 'recording' ? s.stop() : s.start())}>
        <span className="record-icon" />{s.phase === 'recording' ? 'Stop recording' : s.active || s.phase === 'warming' ? `${status}…` : 'Start recording'}
      </button>}
    </div></div>
  </section>;
}
