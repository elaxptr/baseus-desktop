import { createSignal, onCleanup, onMount } from 'solid-js';
import Sidebar, { type Tab } from './components/Sidebar';
import HomeTab from './components/HomeTab';
import AncTab from './components/AncTab';
import EqTab from './components/EqTab';
import GesturesTab from './components/GesturesTab';
import SettingsTab from './components/SettingsTab';
import { onDeviceEvent, onConnectionState, setAncMode, type AncMode, type WearState } from './lib/tauri';
import { pushLeft, pushRight, pushCase, left, right, caseData } from './stores/batteryHistory';
import { loadSettings, getSettingsStore } from './stores/settings';
import { startTimer, stopTimer, useElapsed } from './lib/timer';

type ConnStatus = 'connected' | 'connecting' | 'disconnected';

export default function App() {
  const [status, setStatus] = createSignal<ConnStatus>('connecting');
  const [ancMode, setAncModeSignal] = createSignal<AncMode>('off');
  const [ancLoading, setAncLoading] = createSignal<AncMode | null>(null);
  const [ancLevel, setAncLevel] = createSignal(7);
  const [activeTab, setActiveTab] = createSignal<Tab>('home');
  const [leftCharging, setLeftCharging] = createSignal(false);
  const [rightCharging, setRightCharging] = createSignal(false);
  const [caseCharging, setCaseCharging] = createSignal(false);
  const [wear, setWear] = createSignal<WearState | null>(null);

  onMount(async () => {
    const unlisteners: Array<() => void> = [];
    onCleanup(() => unlisteners.forEach((fn) => fn()));

    await loadSettings();

    onDeviceEvent((e) => {
      if (e.type === 'battery_update') {
        pushLeft(e.data.left_pct);
        pushRight(e.data.right_pct);
        setLeftCharging(e.data.left_charging);
        setRightCharging(e.data.right_charging);
      } else if (e.type === 'case_update') {
        pushCase(e.data.case_pct);
        setCaseCharging(e.data.case_charging);
      } else if (e.type === 'anc_mode_update') {
        setAncModeSignal(e.data);
        setAncLoading(null);
      } else if (e.type === 'wear_update') {
        setWear(e.data);
      }
    }).then((fn) => unlisteners.push(fn));

    onConnectionState((s) => {
      setStatus(s);
      if (s === 'connected') startTimer();
      else stopTimer();
    }).then((fn) => unlisteners.push(fn));
  });

  async function handleAnc(mode: AncMode) {
    if (ancMode() === mode) return;
    setAncLoading(mode);
    const byte = Math.round(((ancLevel() - 1) / 9) * (0xff - 0x10) + 0x10);
    try {
      await setAncMode(mode, mode === 'off' ? undefined : byte);
    } catch {
      setAncLoading(null);
    }
  }

  async function handleLevel(v: number) {
    setAncLevel(v);
    const mode = ancMode();
    if (mode !== 'off') {
      const byte = Math.round(((v - 1) / 9) * (0xff - 0x10) + 0x10);
      await setAncMode(mode, byte).catch(() => {});
    }
  }

  const statusColor = () =>
    status() === 'connected' ? '#22c55e' : status() === 'connecting' ? '#eab308' : '#525252';

  const statusText = () =>
    status() === 'connected' ? 'Connected' : status() === 'connecting' ? 'Connecting…' : 'Disconnected';

  const tab = (id: Tab) => ({ display: activeTab() === id ? 'block' : 'none' });

  return (
    <div
      style={{
        width: '480px',
        'min-height': '620px',
        background: '#0d0d0f',
        color: '#fff',
        'font-family': "-apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif",
        'box-sizing': 'border-box',
        display: 'flex',
        'flex-direction': 'column',
      }}
    >
      {/* Title bar */}
      <div
        style={{
          display: 'flex',
          'align-items': 'center',
          gap: '6px',
          padding: '12px 16px 10px',
          'border-bottom': '1px solid #161618',
          'flex-shrink': '0',
        }}
      >
        <div style={{ display: 'flex', gap: '5px' }}>
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
            color: '#444',
            'margin-left': '-40px',
          }}
        >
          Bass BP1 Pro ANC
        </div>
        <div style={{ display: 'flex', 'align-items': 'center', gap: '5px', 'font-size': '11px', color: statusColor(), 'font-weight': '500' }}>
          <div style={{ width: '6px', height: '6px', background: statusColor(), 'border-radius': '50%' }} />
          {statusText()}
        </div>
      </div>

      {/* Body: sidebar + content */}
      <div style={{ display: 'flex', flex: '1' }}>
        <Sidebar active={activeTab()} onSwitch={setActiveTab} />

        <div style={{ flex: '1', padding: '16px', 'overflow-y': 'auto' }}>
          <div style={tab('home')}>
            <HomeTab
              leftPct={left()[left().length - 1]?.pct ?? 0}
              rightPct={right()[right().length - 1]?.pct ?? 0}
              casePct={caseData()[caseData().length - 1]?.pct ?? 0}
              leftCharging={leftCharging()}
              rightCharging={rightCharging()}
              caseCharging={caseCharging()}
              leftInEar={wear()?.left_in_ear ?? false}
              rightInEar={wear()?.right_in_ear ?? false}
              wearKnown={wear() !== null}
              leftHistory={left().map((r) => r.pct)}
              rightHistory={right().map((r) => r.pct)}
              elapsed={useElapsed()()}
              showTimer={getSettingsStore().show_session_timer}
            />
          </div>

          <div style={tab('anc')}>
            <AncTab
              mode={ancMode()}
              loading={ancLoading()}
              level={ancLevel()}
              onMode={handleAnc}
              onLevel={handleLevel}
            />
          </div>

          <div style={tab('eq')}>
            <EqTab />
          </div>

          <div style={tab('gestures')}>
            <GesturesTab />
          </div>

          <div style={tab('settings')}>
            <SettingsTab />
          </div>
        </div>
      </div>
    </div>
  );
}
