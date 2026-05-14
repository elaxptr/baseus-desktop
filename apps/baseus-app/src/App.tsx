import { createSignal, onCleanup, onMount } from 'solid-js';
import BudCard from './components/BudCard';
import CaseCard from './components/CaseCard';
import AncButton from './components/AncButton';
import FindButton from './components/FindButton';
import SettingRow from './components/SettingRow';
import {
  onDeviceEvent,
  onConnectionState,
  setAncMode,
  findEarbud,
} from './lib/tauri';
import { pushLeft, pushRight, pushCase, left, right, caseData } from './stores/batteryHistory';
import { loadSettings, getSettingsStore, updateSetting } from './stores/settings';

type ConnStatus = 'connected' | 'connecting' | 'disconnected';
type AncMode = 'off' | 'anc' | 'transparency';

export default function App() {
  const [status, setStatus] = createSignal<ConnStatus>('connecting');
  const [leftPct, setLeftPct] = createSignal(0);
  const [rightPct, setRightPct] = createSignal(0);
  const [casePct, setCasePct] = createSignal(0);
  const [ancMode, setAncModeSignal] = createSignal<AncMode>('off');
  const [ancLoading, setAncLoading] = createSignal<AncMode | null>(null);

  onMount(async () => {
    await loadSettings();

    const unlisteners: Array<() => void> = [];
    onCleanup(() => unlisteners.forEach((fn) => fn()));

    onDeviceEvent((e) => {
      if (e.type === 'battery_update') {
        setLeftPct(e.data.left_pct);
        setRightPct(e.data.right_pct);
        pushLeft(e.data.left_pct);
        pushRight(e.data.right_pct);
      } else if (e.type === 'case_update') {
        setCasePct(e.data.case_pct);
        pushCase(e.data.case_pct);
      } else if (e.type === 'anc_mode_update') {
        setAncModeSignal(e.data);
        setAncLoading(null);
      }
    }).then((fn) => unlisteners.push(fn));

    onConnectionState((s) => setStatus(s)).then((fn) => unlisteners.push(fn));
  });

  async function handleAnc(mode: AncMode) {
    if (ancMode() === mode) return;
    setAncLoading(mode);
    try {
      await setAncMode(mode);
    } catch {
      setAncLoading(null);
    }
  }

  const statusColor = () =>
    status() === 'connected' ? '#22c55e' : status() === 'connecting' ? '#eab308' : '#525252';

  const statusText = () =>
    status() === 'connected' ? 'Connected' : status() === 'connecting' ? 'Connecting…' : 'Disconnected';

  const sectionLabelStyle = {
    'font-size': '10px',
    'font-weight': '600',
    color: '#333',
    'letter-spacing': '0.1em',
    'text-transform': 'uppercase',
    'margin-bottom': '14px',
    display: 'flex',
    'align-items': 'center',
    gap: '8px',
  };

  return (
    <div
      style={{
        width: '380px',
        'min-height': '620px',
        background: '#0d0d0f',
        color: '#fff',
        'font-family': "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
        'box-sizing': 'border-box',
      }}
    >
      {/* Title bar */}
      <div
        style={{
          display: 'flex',
          'align-items': 'center',
          gap: '6px',
          padding: '14px 18px 12px',
          'border-bottom': '1px solid #161618',
        }}
      >
        <div style={{ display: 'flex', gap: '6px' }}>
          {(['#ff5f57', '#ffbd2e', '#28c840'] as const).map((c) => (
            <div style={{ width: '11px', height: '11px', 'border-radius': '50%', background: c }} />
          ))}
        </div>
        <div
          style={{
            flex: '1',
            'text-align': 'center',
            'font-size': '12px',
            'font-weight': '600',
            color: '#525252',
            'margin-left': '-50px',
          }}
        >
          Bass BP1 Pro ANC
        </div>
        <div style={{ display: 'flex', 'align-items': 'center', gap: '5px', 'font-size': '11px', color: statusColor(), 'font-weight': '500' }}>
          <div style={{ width: '6px', height: '6px', background: statusColor(), 'border-radius': '50%' }} />
          {statusText()}
        </div>
      </div>

      {/* Earbuds */}
      <div style={{ padding: '20px 20px 0' }}>
        <div style={sectionLabelStyle}>
          Earbuds
          <div style={{ flex: '1', height: '1px', background: '#161618' }} />
        </div>
        <div style={{ display: 'flex', gap: '12px' }}>
          <BudCard
            label="LEFT"
            pct={leftPct()}
            history={left().map((r) => r.pct)}
          />
          <BudCard
            label="RIGHT"
            pct={rightPct()}
            history={right().map((r) => r.pct)}
          />
        </div>
      </div>

      {/* Case */}
      <div style={{ padding: '14px 20px 0' }}>
        <div style={sectionLabelStyle}>
          Case
          <div style={{ flex: '1', height: '1px', background: '#161618' }} />
        </div>
        <CaseCard
          pct={casePct()}
          history={caseData().map((r) => r.pct)}
        />
      </div>

      {/* Noise Control */}
      <div style={{ padding: '14px 20px 0' }}>
        <div style={sectionLabelStyle}>
          Noise Control
          <div style={{ flex: '1', height: '1px', background: '#161618' }} />
        </div>
        <div style={{ display: 'flex', gap: '6px' }}>
          {(
            [
              { mode: 'off' as AncMode, icon: '🔇', label: 'Off' },
              { mode: 'anc' as AncMode, icon: '🎧', label: 'ANC' },
              { mode: 'transparency' as AncMode, icon: '🌬️', label: 'Transparent' },
            ] as const
          ).map(({ mode, icon, label }) => (
            <AncButton
              icon={icon}
              label={label}
              active={ancMode() === mode}
              loading={ancLoading() === mode}
              onClick={() => handleAnc(mode)}
            />
          ))}
        </div>
      </div>

      {/* Find Earbuds */}
      <div style={{ padding: '14px 20px 0' }}>
        <div style={sectionLabelStyle}>
          Find Earbuds
          <div style={{ flex: '1', height: '1px', background: '#161618' }} />
        </div>
        <div style={{ display: 'flex', gap: '8px' }}>
          <FindButton label="Play Left" onClick={() => findEarbud('left').catch(() => {})} />
          <FindButton label="Play Right" onClick={() => findEarbud('right').catch(() => {})} />
        </div>
      </div>

      {/* Settings */}
      <div style={{ padding: '14px 20px 20px' }}>
        <div style={sectionLabelStyle}>
          Settings
          <div style={{ flex: '1', height: '1px', background: '#161618' }} />
        </div>
        <SettingRow
          label="Launch at login"
          description="Start automatically with Windows"
          value={getSettingsStore().launch_at_login}
          onChange={(v) => updateSetting('launch_at_login', v)}
        />
        <div style={{ height: '1px', background: '#131315' }} />
        <SettingRow
          label="Low battery alerts"
          description="Notify when a bud drops below 20%"
          value={getSettingsStore().low_battery_alerts}
          onChange={(v) => updateSetting('low_battery_alerts', v)}
        />
      </div>
    </div>
  );
}
