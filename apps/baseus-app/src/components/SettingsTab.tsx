import { createSignal, onMount, Show } from 'solid-js';
import { getSettingsStore, updateSetting } from '../stores/settings';
import { checkForUpdate, installUpdate } from '../lib/tauri';
import { getVersion } from '@tauri-apps/api/app';

interface Props {
  initialUpdateVersion: string | null;
  onUpdateInstalled: () => void;
}

type UpdateState = 'idle' | 'checking' | 'up-to-date' | 'available' | 'installing' | 'error';

export default function SettingsTab(props: Props) {
  const [appVersion, setAppVersion] = createSignal('');
  const [updateState, setUpdateState] = createSignal<UpdateState>('idle');
  const [availableVersion, setAvailableVersion] = createSignal<string | null>(props.initialUpdateVersion);
  // Last check/install failure. Shown until the next attempt so a failure isn't silent.
  const [updateError, setUpdateError] = createSignal<string | null>(null);

  onMount(async () => {
    setAppVersion(await getVersion());
    // If the background check already found an update, reflect that immediately.
    if (props.initialUpdateVersion) setUpdateState('available');
  });

  async function handleCheck() {
    setUpdateState('checking');
    setUpdateError(null);
    try {
      const version = await checkForUpdate();
      if (version) {
        setAvailableVersion(version);
        setUpdateState('available');
      } else {
        setUpdateState('up-to-date');
      }
    } catch (e) {
      setUpdateError(`Couldn't check for updates: ${e}`);
      setUpdateState('error');
    }
  }

  async function handleInstall() {
    setUpdateState('installing');
    setUpdateError(null);
    try {
      await installUpdate();
      props.onUpdateInstalled();
    } catch (e) {
      setUpdateError(`Couldn't install the update: ${e}`);
      setUpdateState('available');
    }
  }

  const busy = () => updateState() === 'checking' || updateState() === 'installing';
  // The install panel stays up while installing; the check button only replaces it after.
  const showInstall = () => updateState() === 'available' || updateState() === 'installing';

  return (
    <div>
      <div style={labelStyle}>Preferences <Divider /></div>
      <Toggle
        label="Launch at login"
        desc="Start automatically when you log in"
        value={getSettingsStore().launch_at_login}
        onChange={(v) => updateSetting('launch_at_login', v)}
      />
      <div style={{ height: '1px', background: '#131315' }} />
      <Toggle
        label="Low battery alerts"
        desc="Notify when a bud drops below 20%"
        value={getSettingsStore().low_battery_alerts}
        onChange={(v) => updateSetting('low_battery_alerts', v)}
      />
      <div style={{ height: '1px', background: '#131315' }} />
      <Toggle
        label="Show session timer"
        desc="Display listening time on the home tab"
        value={getSettingsStore().show_session_timer}
        onChange={(v) => updateSetting('show_session_timer', v)}
      />

      <div style={{ 'margin-top': '20px' }}>
        <div style={labelStyle}>Software <Divider /></div>
        <div
          style={{
            background: '#111113',
            border: '1px solid #1a1a1e',
            'border-radius': '12px',
            padding: '14px 16px',
          }}
        >
          <div style={{ display: 'flex', 'align-items': 'center', 'justify-content': 'space-between' }}>
            <div>
              <div style={{ 'font-size': '13px', color: '#ccc', 'font-weight': '500' }}>
                baseus-desktop
              </div>
              <div style={{ 'font-size': '10px', color: '#444', 'margin-top': '2px' }}>
                {appVersion() ? `v${appVersion()}` : '…'}
              </div>
            </div>

            <Show when={!showInstall()}>
              <button
                onClick={handleCheck}
                disabled={busy()}
                style={{
                  background: 'transparent',
                  border: '1px solid #2a2a2e',
                  'border-radius': '8px',
                  padding: '5px 10px',
                  'font-size': '11px',
                  color: busy() ? '#444' : '#888',
                  cursor: busy() ? 'default' : 'pointer',
                }}
              >
                {updateState() === 'checking' ? 'Checking…' :
                 updateState() === 'up-to-date' ? '✓ Up to date' :
                 updateState() === 'error' ? 'Retry' : 'Check for updates'}
              </button>
            </Show>
          </div>

          <Show when={showInstall()}>
            <div
              style={{
                'margin-top': '12px',
                padding: '10px 12px',
                background: 'rgba(34,197,94,0.06)',
                border: '1px solid rgba(34,197,94,0.2)',
                'border-radius': '8px',
                display: 'flex',
                'align-items': 'center',
                'justify-content': 'space-between',
                gap: '8px',
              }}
            >
              <div>
                <div style={{ 'font-size': '12px', color: '#4ade80', 'font-weight': '600' }}>
                  Update available
                </div>
                <div style={{ 'font-size': '10px', color: '#444', 'margin-top': '2px' }}>
                  v{availableVersion()}
                </div>
              </div>
              <button
                onClick={handleInstall}
                disabled={updateState() === 'installing'}
                style={{
                  background: 'rgba(34,197,94,0.15)',
                  border: '1px solid rgba(34,197,94,0.3)',
                  'border-radius': '8px',
                  padding: '5px 12px',
                  'font-size': '11px',
                  'font-weight': '600',
                  color: updateState() === 'installing' ? '#444' : '#4ade80',
                  cursor: updateState() === 'installing' ? 'default' : 'pointer',
                  'white-space': 'nowrap',
                }}
              >
                {updateState() === 'installing' ? 'Installing…' : 'Install & Restart'}
              </button>
            </div>
          </Show>

          <Show when={updateError()}>
            <div style={{ 'margin-top': '10px', 'font-size': '11px', color: '#f87171', 'word-break': 'break-word' }}>
              {updateError()}
            </div>
          </Show>
        </div>
      </div>
    </div>
  );
}

function Toggle(p: { label: string; desc: string; value: boolean; onChange: (v: boolean) => void }) {
  return (
    <div style={{ display: 'flex', 'align-items': 'center', padding: '14px 0' }}>
      <div style={{ flex: '1' }}>
        <div style={{ 'font-size': '13px', color: '#ccc', 'font-weight': '500' }}>{p.label}</div>
        <div style={{ 'font-size': '10px', color: '#444', 'margin-top': '2px' }}>{p.desc}</div>
      </div>
      <div
        onClick={() => p.onChange(!p.value)}
        style={{
          width: '36px', height: '20px',
          'border-radius': '99px',
          background: p.value ? '#22c55e' : '#222',
          position: 'relative',
          cursor: 'pointer',
          transition: 'background 0.2s',
          'flex-shrink': '0',
        }}
      >
        <div
          style={{
            position: 'absolute',
            width: '16px', height: '16px',
            'border-radius': '50%',
            background: '#fff',
            top: '2px',
            left: p.value ? 'auto' : '2px',
            right: p.value ? '2px' : 'auto',
            transition: 'left 0.2s, right 0.2s',
            'box-shadow': '0 1px 4px rgba(0,0,0,0.4)',
          }}
        />
      </div>
    </div>
  );
}

function Divider() {
  return <div style={{ flex: '1', height: '1px', background: '#161618' }} />;
}

const labelStyle = {
  'font-size': '9px',
  'font-weight': '700',
  color: '#333',
  'letter-spacing': '0.12em',
  'text-transform': 'uppercase' as const,
  display: 'flex',
  'align-items': 'center',
  gap: '8px',
  'margin-bottom': '8px',
};
