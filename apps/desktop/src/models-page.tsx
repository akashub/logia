// claude 2026-09-14: the Models page, backed by the Rust catalogue.
//
// Every row states what is actually true of that artifact: whether it is
// verified on disk, whether it can produce live captions, and whether its
// download details are pinned yet. An entry that is planned but not pinned is
// shown as such rather than hidden — the reader can see what is coming without
// being offered a download that would fail.
import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export type CatalogEntry = {
  id: string; name: string; detail: string; languages: string;
  bytes: number; family: string; installed: boolean; active: boolean; available: boolean; streams: boolean;
};

type Props = {
  native: boolean; phase: string; visible: boolean; busy: boolean;
  onPrepare: () => void; onError: (message: string) => void;
};

function size(bytes: number) {
  return bytes > 0 ? `${Math.round(bytes / 1_000_000)} MB` : 'Size not pinned';
}

export function ModelsPage({ native, phase, visible, busy, onPrepare, onError }: Props) {
  const [models, setModels] = useState<CatalogEntry[]>([]);
  const [working, setWorking] = useState('');
  const [downloading, setDownloading] = useState(''), [downloadProgress, setDownloadProgress] = useState(0);
  const revision = useRef(0);

  const refresh = useCallback(async () => {
    if (!native) return;
    const request = ++revision.current;
    try {
      const next = await invoke<CatalogEntry[]>('list_models');
      if (request === revision.current) setModels(next);
    }
    catch (e) { onError(String(e)); }
  }, [native, onError]);

  useEffect(() => { if (visible) void refresh(); }, [refresh, phase, visible]);
  useEffect(() => {
    if (!native) return;
    const off = listen<number>('model-progress', ({ payload }) => setDownloadProgress(payload));
    return () => { void off.then(unsubscribe => unsubscribe()); };
  }, [native]);

  async function install(id: string) {
    revision.current++; setWorking(id); setDownloading(id); setDownloadProgress(0); onError('');
    try {
      await invoke('download_model', { id }); await refresh();
      if (models.find(model => model.id === id)?.active) onPrepare();
    }
    catch (e) { onError(String(e)); }
    finally { setWorking(''); setDownloading(''); }
  }
  async function select(id: string) {
    revision.current++; setWorking(id); onError('');
    try { await invoke('select_model', { id }); await refresh(); onPrepare(); }
    catch (e) { onError(String(e)); }
    finally { setWorking(''); }
  }
  async function remove(id: string) {
    revision.current++; setWorking(id); onError('');
    try { await invoke('remove_model', { id }); await refresh(); }
    catch (e) { onError(String(e)); }
    finally { setWorking(''); }
  }

  return <section className="settings-models" hidden={!visible}>
    {models.length === 0 && <p className="settings-endnote">Reading the model list…</p>}
    {models.map(model => {
      const isDownloading = downloading === model.id;
      const disabled = !native || busy || Boolean(working);
      return <article key={model.id} className="settings-model-row" data-installed={model.installed ? 'yes' : 'no'}>
        <div className="settings-model-head">
          <h2>{model.name}</h2>
          <span className={`settings-badge ${model.installed ? 'is-installed' : ''}`}>
            {model.active && model.installed ? 'Active' : model.installed ? 'Installed' : model.available ? 'Available' : 'Not yet available'}
          </span>
        </div>
        <p className="settings-model-meta">
          {model.languages}<span>{size(model.bytes)}</span>
          <span>{model.streams ? 'Live captions' : 'No live captions'}</span>
        </p>
        <p>{model.detail}</p>

        {isDownloading && <div className="settings-download">
          <progress aria-label={`Downloading ${model.name}`} max="100" value={downloadProgress} /><span>{downloadProgress}%</span>
        </div>}

        <div className="settings-model-actions">
          {model.installed
            ? <>
                {model.active
                  ? <button className="settings-button" disabled={disabled} onClick={onPrepare}>Prepare again</button>
                  : <button className="settings-button settings-button-primary" disabled={disabled || !model.available}
                      onClick={() => void select(model.id)}>Use model</button>}
                <button className="settings-button" disabled={disabled || model.active}
                  onClick={() => void remove(model.id)}>Remove</button>
              </>
            : <button className="settings-button settings-button-primary"
                disabled={disabled || !model.available}
                onClick={() => void install(model.id)}>
                {isDownloading ? 'Downloading…' : model.available ? 'Download' : 'Not yet available'}
              </button>}
        </div>

        {!model.available && <p className="settings-model-note">
          {model.streams ? 'Its download details are not pinned yet.' : 'This variant cannot provide live captions in the current runtime.'}
        </p>}
      </article>;
    })}
    {phase === 'warming' && <p className="settings-model-note">Preparing local recognition. Your microphone is off.</p>}
    <p className="settings-endnote">
      Every model is verified by exact size and SHA-256 before it is used. Switching models is only
      possible while no recording is running.
    </p>
  </section>;
}
