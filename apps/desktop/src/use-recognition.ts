import { useEffect, type MutableRefObject } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Phase } from './dictation-types';

type Update = { generation: number; event: { type: string; text?: string; message?: string; peak?: number } };
type Options = {
  native: boolean; generation: MutableRefObject<number>; phase: MutableRefObject<Phase>;
  transition: (phase: Phase) => void; text: (text: string) => void; error: (error: string) => void;
  complete: (complete: boolean) => void; seconds: (seconds: number) => void;
  level: (peak: number) => void;
  progress: (progress: number) => void; prepare: () => Promise<void>;
};
export function useRecognition(options: Options) {
  const { native, generation, phase, transition, text, error, complete, seconds, progress, prepare, level } = options;
  useEffect(() => {
    if (!native) return;
    let disposed = false;
    const subscriptions: (() => void)[] = [];
    async function connect() {
      const updates = await listen<Update>('recognition', ({ payload }) => {
        if (disposed || payload.generation < generation.current) return;
        generation.current = payload.generation;
        // Cancel's command resolves only after owned-worker reap and gives the
        // authoritative outcome. A preceding Stopped event cannot open a gap.
        if (phase.current === 'canceling') return;
        const event = payload.event;
        if (event.type === 'loading' && phase.current !== 'warming') transition('loading');
        if (event.type === 'listening') { transition('recording'); seconds(0); level(0); }
        // claude 2026-09-11: a measured peak. Zero for the whole session means the
        // device is handing us silence, which is what a denied microphone does.
        if (event.type === 'level' && typeof event.peak === 'number') level(event.peak);
        if (event.type === 'partial' && event.text?.trim()) text(event.text);
        if (event.type === 'final') { text(event.text ?? ''); complete(true); transition('finishing'); }
        if (event.type === 'error') error(event.message ?? 'Recognition stopped. Try again.');
        if (event.type === 'stopped') transition('ready');
      });
      if (disposed) { updates(); return; }
      subscriptions.push(updates);
      const download = await listen<number>('model-progress', ({ payload }) => { if (!disposed) progress(payload); });
      if (disposed) { download(); return; }
      subscriptions.push(download);
      try { if (await invoke<boolean>('model_ready')) await prepare(); else transition('setup'); }
      catch (e) { error(String(e)); transition('setup'); }
    }
    void connect().catch(() => {
      if (!disposed) { error('Could not connect to recognition. Quit and reopen Logia.'); transition('setup'); }
    });
    return () => { disposed = true; subscriptions.forEach(unsubscribe => unsubscribe()); };
  }, [native]);
}
