import { invoke } from '@tauri-apps/api/core';
import { useCallback, useEffect, useRef, useState } from 'react';

export type AudioInput = { id: string; name: string };
type Catalogue = { devices: AudioInput[]; selected: AudioInput | null; default_id: string | null; error: string | null };

// Owned by Settings, so navigating away never abandons an in-flight save.
export function useAudioInputs(native: boolean, visible: boolean, busy: boolean) {
  const [data, setData] = useState<Catalogue | null>(null);
  const [saving, setSaving] = useState(false);
  const [readError, setReadError] = useState('');
  const [saveError, setSaveError] = useState('');
  const mounted = useRef(false);
  const sequence = useRef(0);
  const reading = useRef<number | null>(null);
  const writing = useRef(false);
  const disabled = useRef(busy);
  disabled.current = busy;
  useEffect(() => {
    mounted.current = true;
    return () => { mounted.current = false; sequence.current++; reading.current = null; };
  }, []);

  const refresh = useCallback(async () => {
    if (!native || writing.current || reading.current !== null) return;
    const ticket = ++sequence.current;
    reading.current = ticket;
    try {
      const result = await invoke<Catalogue>('list_audio_inputs');
      if (mounted.current && ticket === sequence.current) { setData(result); setReadError(''); }
    } catch (error) {
      if (mounted.current && ticket === sequence.current) setReadError(String(error));
    } finally {
      if (reading.current === ticket) reading.current = null;
    }
  }, [native]);

  useEffect(() => {
    if (!visible || busy) return;
    const check = () => { if (document.visibilityState === 'visible') void refresh(); };
    check();
    const timer = window.setInterval(check, 3000);
    window.addEventListener('focus', check);
    document.addEventListener('visibilitychange', check);
    return () => {
      clearInterval(timer);
      window.removeEventListener('focus', check);
      document.removeEventListener('visibilitychange', check);
    };
  }, [visible, busy, refresh]);

  async function choose(id: string) {
    if (!native || disabled.current || writing.current) return;
    writing.current = true; sequence.current++; reading.current = null;
    setSaving(true); setSaveError('');
    try {
      const selected = await invoke<AudioInput | null>('select_audio_input', { id: id || null });
      if (mounted.current) setData(previous => ({
        devices: previous?.devices ?? [], default_id: previous?.default_id ?? null,
        selected, error: null,
      }));
    } catch (error) {
      if (mounted.current) setSaveError(String(error));
    } finally {
      writing.current = false;
      if (mounted.current) { setSaving(false); void refresh(); }
    }
  }
  return { data, saving, error: saveError || readError || data?.error || '', refresh, choose };
}
