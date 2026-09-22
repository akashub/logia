export type Phase = 'checking' | 'setup' | 'downloading' | 'warming' | 'ready' | 'loading' | 'recording' | 'finishing' | 'canceling';
export type DeliveryStatus = 'copy' | 'permission' | 'armed' | 'sending' | 'sent' | 'dispatched' | 'clipboard-changed' | 'copied' | 'copy-required' | 'uncertain';
export type OverlayAction = 'stop' | 'cancel' | 'copy' | 'edit' | 'dismiss' | 'permissions';
export type OverlayRequest = { action: OverlayAction; session: number; text?: string };
export type OverlaySnapshot = {
  // `seq` increments on every published snapshot. The overlay echoes it back in
  // `overlay-applied` once the state is rendered, which is the only evidence the
  // panel actually shows what the user is about to speak into. A successful
  // emit proves delivery to the event system, not that anything was drawn.
  seq: number;
  session: number; phase: Phase; text: string; complete: boolean; error: string;
  seconds: number; delivery: DeliveryStatus; copied: boolean; dismissed: boolean;
  /** Measured peak amplitude, 0..1, from the worker's capture. Not simulated. */
  level: number;
  /** Recording for several seconds with nothing but digital silence. */
  deaf: boolean;
  shortcut: string; position: 'top' | 'bottom'; showIdle: boolean;
};
export const activePhase = (phase: Phase) => ['loading', 'recording', 'finishing', 'canceling'].includes(phase);
