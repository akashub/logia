import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

// Native focus is opt-in after recovery. A delayed response cannot make a new
// recording editable, and the native command separately rejects active workers.
export function useOverlayEditing(allowed: boolean, session: number) {
  const [editing, setEditing] = useState(false), [error, setError] = useState('');
  const intent = useRef(0), pending = useRef(false);
  const permit = useRef<number | null>(null);
  const current = useRef({ allowed, session });
  current.current = { allowed, session };

  useEffect(() => {
    const request = ++intent.current;
    permit.current = null;
    setEditing(false); setError('');
    void invoke('set_overlay_editing', { editing: false }).catch(() => {});
    if (allowed) void invoke<number>('overlay_editing_token').then(epoch => {
      if (request === intent.current) permit.current = epoch;
    }).catch(() => { if (request === intent.current) setError('Open Recovery in Logia to edit this text.'); });
    return () => {
      intent.current++;
      void invoke('set_overlay_editing', { editing: false }).catch(() => {});
    };
  }, [allowed, session]);

  async function begin() {
    if (!allowed || editing || pending.current || permit.current === null) return;
    const request = ++intent.current;
    pending.current = true; setError('');
    try {
      await invoke('set_overlay_editing', { editing: true, epoch: permit.current });
      if (request !== intent.current || !current.current.allowed || current.current.session !== session) {
        await invoke('set_overlay_editing', { editing: false });
        return;
      }
      setEditing(true);
    } catch {
      if (request === intent.current) setError('Open Recovery in Logia to edit this text.');
    } finally { pending.current = false; }
  }
  return { editing: editing && allowed, error, begin };
}
