import type { useAudioInputs } from './use-audio-inputs';

type Props = { inputs: ReturnType<typeof useAudioInputs>; disabled: boolean };
export function MicrophonePicker({ inputs, disabled }: Props) {
  const { data, saving, error, choose, refresh } = inputs;
  const devices = data?.devices ?? [];
  const missing = data?.selected && !devices.some(device => device.id === data.selected?.id);
  const defaultName = devices.find(device => device.id === data?.default_id)?.name;
  const unknown = !!data?.error;
  const value = unknown ? '__invalid__' : data?.selected?.id ?? '';
  const detail = missing ? 'Reconnect it or choose another microphone.'
    : saving ? 'Saving microphone choice…'
    : !data ? 'Choose which microphone Logia uses for dictation.'
    : devices.length === 0 ? 'No microphones connected.'
    : !data.selected && defaultName ? `Currently using ${defaultName}.`
    : 'Used for global dictation and voice tests.';
  return <>
    <div className="settings-row settings-microphone">
      <div className="settings-row-copy"><h3>Microphone</h3><p id="microphone-detail">{detail}</p></div>
      <div className="settings-row-control">
        <select aria-label="Microphone" aria-describedby="microphone-detail" value={value}
          disabled={disabled || saving || !data} onChange={event => void choose(event.target.value)}>
          {unknown && <option value="__invalid__" disabled>Choose microphone</option>}
          <option value="">System default</option>
          {missing && <option value={data!.selected!.id} disabled>{data!.selected!.name} — unavailable</option>}
          {devices.map(device => {
            const sameName = devices.filter(other => other.name === device.name);
            const suffix = sameName.length > 1 ? ` (${sameName.findIndex(other => other.id === device.id) + 1})` : '';
            return <option key={device.id} value={device.id}>{device.name}{suffix}</option>;
          })}
        </select>
      </div>
    </div>
    {error && <div className="settings-inline-error" role="alert">{error}
      {!data && <button className="settings-button" disabled={disabled} onClick={() => void refresh()}>Try again</button>}
    </div>}
  </>;
}
