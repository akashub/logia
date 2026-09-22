// claude 2026-09-14: the Models page, backed by the Rust catalogue.
//
// Every row states what is actually true of that artifact: whether it is
// verified on disk, whether it can produce live captions, and whether its
// download details are pinned yet. An entry that is planned but not pinned is
// shown as such rather than hidden — the reader can see what is coming without
// being offered a download that would fail.
import { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

export type CatalogEntry = {
  id: string; name: string; detail: string; languages: string;
  bytes: number; family: string; installed: boolean; available: boolean; streams: boolean;
};

type Props = {
  native: boolean; phase: string; progress: number; busy: boolean;
  onPrepare: () => void; onError: (message: string) => void;
};

function size(bytes: number) {
  return bytes > 0 ? `${Math.round(bytes / 1_000_000)} MB` : 'Size not pinned';
}

export function ModelsPage({ native, phase, progress, busy, onPrepare, onError }: Props) {
  const [models, setModels] = useState<CatalogEntry[]>([]);
  const [working, setWorking] = useState('');

  const refresh = useCallback(async () => {
    if (!native) return;
    try { setModels(await invoke<CatalogEntry[]>('list_models')); }
    catch (e) { onError(String(e)); }
  }, [native, onError]);

  useEffect(() => { void refresh(); }, [refresh, phase]);

  async function install(id: string) {
    setWorking(id);
    try { await invoke('download_model', { id }); await refresh(); onPrepare(); }
    catch (e) { onError(String(e)); }
    finally { setWorking(''); }
  }
  async function remove(id: string) {
    setWorking(id);
    try { await invoke('remove_model', { id }); await refresh(); }
    catch (e) { onError(String(e)); }
    finally { setWorking(''); }
  }

  return <section className="settings-models">
    {models.length === 0 && <p className="settings-endnote">Reading the model list…</p>}
    {models.map(model => {
      const downloading = phase === 'downloading' && working === model.id;
      return <article key={model.id} className="settings-model-row" data-installed={model.installed ? 'yes' : 'no'}>
        <div className="settings-model-head">
          <h2>{model.name}</h2>
          <span className={`settings-badge ${model.installed ? 'is-installed' : ''}`}>
            {model.installed ? 'Installed' : model.available ? 'Available' : 'Not yet available'}
          </span>
        </div>
        <p className="settings-model-meta">
          {model.languages}<span>{size(model.bytes)}</span>
          <span>{model.streams ? 'Live captions' : 'No live captions'}</span>
        </p>
        <p>{model.detail}</p>

        {downloading && <div className="settings-download">
          <progress aria-label={`Downloading ${model.name}`} max="100" value={progress} /><span>{progress}%</span>
        </div>}

        <div className="settings-model-actions">
          {model.installed
            ? <>
                <button className="settings-button" disabled={!native || busy} onClick={onPrepare}>Prepare again</button>
                <button className="settings-button" disabled={!native || Boolean(working)}
                  onClick={() => void remove(model.id)}>Remove</button>
              </>
            : <button className="settings-button settings-button-primary"
                disabled={!native || !model.available || Boolean(working) || busy}
                onClick={() => void install(model.id)}>
                {downloading ? 'Downloading…' : model.available ? 'Download' : 'Not yet available'}
              </button>}
        </div>

        {!model.available && <p className="settings-model-note">
          Its download details are not pinned yet, so Logia will not offer an artifact it cannot verify.
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
