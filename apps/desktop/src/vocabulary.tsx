// claude 2026-09-11: R07 vocabulary UI.
//
// Rules are exact spoken-phrase → replacement pairs the user writes themselves.
// Nothing here infers rules, learns from corrections, or edits text on its own:
// the preview calls the same Rust implementation that runs during delivery, so
// what the user is shown is what will actually happen.
import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { issueMessage, ruleIssue, type DictionaryRule } from './dictionary';

const SAMPLE = 'Lobia keeps the use state hook behind a hard key.';

type Props = { rules: DictionaryRule[]; onRules: (rules: DictionaryRule[]) => void; native: boolean };

export function Vocabulary({ rules, onRules, native }: Props) {
  const [spoken, setSpoken] = useState('');
  const [replacement, setReplacement] = useState('');
  const [problem, setProblem] = useState('');
  const [sample, setSample] = useState(SAMPLE);
  const [preview, setPreview] = useState('');

  useEffect(() => {
    if (!native) { setPreview(''); return; }
    let cancelled = false;
    void invoke<string>('preview_dictionary', { text: sample, rules })
      .then(result => { if (!cancelled) setPreview(result); })
      .catch(() => { if (!cancelled) setPreview(''); });
    return () => { cancelled = true; };
  }, [native, sample, rules]);

  function add() {
    const candidate: DictionaryRule = { spoken, replacement, enabled: true };
    const issue = ruleIssue(candidate, rules);
    if (issue) { setProblem(issueMessage(issue)); return; }
    onRules([...rules, { spoken: spoken.trim(), replacement: replacement.trim(), enabled: true }]);
    setSpoken(''); setReplacement(''); setProblem('');
  }

  return <section className="settings-group vocabulary">
    <h2>Vocabulary</h2>
    <p className="vocabulary-intro">
      Recognition mishears names and technical terms. Add the words you actually
      say and exactly what should replace them. Rules apply to the finished
      transcript only, match whole phrases, and never change anything else.
    </p>

    <div className="vocabulary-add">
      <label>When Logia hears
        <input value={spoken} onChange={e => { setSpoken(e.target.value); setProblem(''); }}
          placeholder="lobia" aria-label="Spoken words" maxLength={120} />
      </label>
      <span className="vocabulary-arrow" aria-hidden="true">→</span>
      <label>Write
        <input value={replacement} onChange={e => { setReplacement(e.target.value); setProblem(''); }}
          placeholder="Logia" aria-label="Replacement" maxLength={120}
          onKeyDown={e => { if (e.key === 'Enter') { e.preventDefault(); add(); } }} />
      </label>
      <button className="settings-button settings-button-primary" onClick={add}>Add rule</button>
    </div>
    {problem && <p className="vocabulary-problem" role="alert">{problem}</p>}

    {rules.length === 0
      ? <p className="vocabulary-empty">No rules yet. Dictate something, see what comes out wrong, then add it here.</p>
      : <ul className="vocabulary-list">
          {rules.map((rule, index) => <li key={`${rule.spoken}-${index}`}>
            <span className="vocabulary-spoken">{rule.spoken}</span>
            <span className="vocabulary-arrow" aria-hidden="true">→</span>
            <code className="vocabulary-replacement">{rule.replacement}</code>
            <button type="button" className="settings-switch" role="switch"
              aria-label={`Enable rule ${rule.spoken}`} aria-checked={rule.enabled}
              onClick={() => onRules(rules.map((item, at) => at === index ? { ...item, enabled: !item.enabled } : item))}><span /></button>
            <button type="button" className="settings-button vocabulary-remove"
              aria-label={`Remove rule ${rule.spoken}`}
              onClick={() => onRules(rules.filter((_, at) => at !== index))}>Remove</button>
          </li>)}
        </ul>}

    <div className="vocabulary-preview">
      <label>Try a sentence
        <input value={sample} onChange={e => setSample(e.target.value)} aria-label="Preview sentence" maxLength={300} />
      </label>
      <p aria-live="polite"><span className="vocabulary-preview-label">Becomes</span>
        <span className="vocabulary-preview-text">{preview || sample}</span></p>
    </div>
  </section>;
}
