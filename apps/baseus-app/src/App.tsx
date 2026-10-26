import { createMemo, createSignal, onCleanup, onMount, Show } from 'solid-js';
import Sidebar, { type Tab } from './components/Sidebar';
import HomeTab from './components/HomeTab';
import AncTab from './components/AncTab';
import EqTab from './components/EqTab';
import SettingsTab from './components/SettingsTab';
import {
  getDeviceSnapshot,
  onDeviceEvent,
  onConnectionState,
  onModelInfo,
  onUpdateAvailable,
  setAncMode,
  setEqPreset,
  setGameMode,
  type AncMode,
  type DeviceEvent,
  type EqPreset,
  type ModelInfo,
  type WearState,
} from './lib/tauri';
import { pushLeft, pushRight, pushCase, left, right, caseData } from './stores/batteryHistory';
import { loadSettings, getSettingsStore } from './stores/settings';
import { startTimer, stopTimer, useElapsed } from './lib/timer';

type ConnStatus = 'connected' | 'connecting' | 'disconnected';

const BP1_ANC_MODES: AncMode[] = ['off', 'anc', 'transparency'];
const DEFAULT_STRENGTH_MODES: AncMode[] = ['anc', 'transparency'];
const CONTINUOUS_LEVEL_MAX = 10;

export default function App() {
  const [status, setStatus] = createSignal<ConnStatus>('connecting');
  const [modelInfo, setModelInfo] = createSignal<ModelInfo | null>(null);
  const [ancMode, setAncModeSignal] = createSignal<AncMode>('off');
  const [ancLoading, setAncLoading] = createSignal<AncMode | null>(null);
  const [ancLevel, setAncLevel] = createSignal(7);
  const [gameMode, setGameModeSignal] = createSignal(false);
  const [activeTab, setActiveTab] = createSignal<Tab>('home');
  const [leftCharging, setLeftCharging] = createSignal(false);
  const [rightCharging, setRightCharging] = createSignal(false);
  const [caseCharging, setCaseCharging] = createSignal(false);
  const [wear, setWear] = createSignal<WearState | null>(null);
  const [eqPreset, setEqPresetSignal] = createSignal<EqPreset>('balanced');
  const [updateVersion, setUpdateVersion] = createSignal<string | null>(null);

  const connectedModelName = createMemo(() => modelInfo()?.name ?? 'Bass BP1 Pro ANC');
  const supportedAncModes = createMemo<AncMode[]>(() => BP1_ANC_MODES);
  const strengthModes = createMemo<AncMode[]>(() => modelInfo()?.strength_modes ?? DEFAULT_STRENGTH_MODES);
  const levelMax = createMemo(() => modelInfo()?.anc_levels ?? CONTINUOUS_LEVEL_MAX);

  /// Slider position → level byte. Discrete-level models (BP1 Ultra) take the step
  /// number itself; the BP1 Pro maps 1–10 onto its 0x10–0xFF strength byte.
  function levelByte(v: number): number {
    if (modelInfo()?.anc_levels) return v;
    return Math.round(((v - 1) / 9) * (0xff - 0x10) + 0x10);
  }

  onMount(async () => {
    const unlisteners: Array<() => void> = [];
    onCleanup(() => unlisteners.forEach((fn) => fn()));

    await loadSettings();

    const handleDeviceEvent = (e: DeviceEvent) => {
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
      } else if (e.type === 'anc_level_update') {
        setAncLevel(Math.min(Math.max(e.data, 1), levelMax()));
      } else if (e.type === 'game_mode_update') {
        setGameModeSignal(e.data);
      } else if (e.type === 'wear_update') {
        setWear(e.data);
      } else if (e.type === 'eq_preset_update' && modelInfo()?.eq_state) {
        setEqPresetSignal(e.data);
      }
    };
    const seen = new Set<DeviceEvent['type']>();
    unlisteners.push(
      await onDeviceEvent((e) => {
        // Before the model is known an event may be ignored (EQ) or overwritten by
        // applyModel (ANC mode), so let the snapshot replay below re-apply that kind.
        if (modelInfo() !== null) seen.add(e.type);
        handleDeviceEvent(e);
      }),
    );

    const applyStatus = (s: ConnStatus) => {
      setStatus(s);
      if (s === 'connected') startTimer();
      else stopTimer();
    };
    const applyModel = (info: ModelInfo) => {
      setModelInfo(info);
      setAncModeSignal('off');
      setAncLevel((v) => Math.min(v, info.anc_levels ?? CONTINUOUS_LEVEL_MAX));
    };

    unlisteners.push(await onConnectionState(applyStatus));
    unlisteners.push(await onModelInfo(applyModel));

    // The backend may have connected before these listeners existed (the webview loads
    // in the background); catch up on what was already published. Model first: event
    // handling depends on its capabilities. Skip event kinds a listener already delivered,
    // which are newer than the snapshot's.
    const snapshot = await getDeviceSnapshot().catch(() => null);
    if (snapshot?.model && modelInfo() === null) applyModel(snapshot.model);
    if (snapshot && status() === 'connecting') applyStatus(snapshot.state);
    snapshot?.events.filter((e) => !seen.has(e.type)).forEach(handleDeviceEvent);

    onUpdateAvailable((version) => setUpdateVersion(version))
      .then((fn) => unlisteners.push(fn));
  });

  async function handleAnc(mode: AncMode) {
    if (ancMode() === mode) return;
    setAncLoading(mode);
    try {
      await setAncMode(mode, mode === 'off' ? undefined : levelByte(ancLevel()));
    } catch {
      setAncLoading(null);
    }
  }

  async function handleGameMode(on: boolean) {
    setGameModeSignal(on);
    await setGameMode(on).catch(() => setGameModeSignal(!on));
  }

  async function handleEqPreset(preset: EqPreset) {
    if (eqPreset() === preset) return;
    setEqPresetSignal(preset);
    await setEqPreset(preset).catch(() => {});
  }

  function handleLevel(v: number) {
    setAncLevel(v);
  }

  async function handleLevelCommit(v: number) {
    setAncLevel(v);
    const mode = ancMode();
    if (strengthModes().includes(mode)) {
      await setAncMode(mode, levelByte(v)).catch(() => {});
    }
  }

  const statusColor = () =>
    status() === 'connected' ? '#22c55e' : status() === 'connecting' ? '#eab308' : '#525252';

  const statusText = () =>
    status() === 'connected' ? 'Connected' : status() === 'connecting' ? 'Connecting…' : 'Disconnected';

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
        class="titlebar-glow"
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
          {connectedModelName()}
        </div>
        <div style={{ display: 'flex', 'align-items': 'center', gap: '5px', 'font-size': '11px', color: statusColor(), 'font-weight': '500' }}>
          <div class={status() === 'connected' ? 'status-dot' : undefined} style={{ width: '6px', height: '6px', background: statusColor(), 'border-radius': '50%' }} />
          {statusText()}
        </div>
      </div>

      {/* Body: sidebar + content */}
      <div style={{ display: 'flex', flex: '1' }}>
        <Sidebar active={activeTab()} onSwitch={setActiveTab} updateAvailable={updateVersion() !== null} />

        <div style={{ flex: '1', padding: '16px', 'overflow-y': 'auto' }}>
          <Show when={activeTab() === 'home'}>
            <div class="panel-in">
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
                showFind={modelInfo()?.find_earbud ?? true}
              />
            </div>
          </Show>

          <Show when={activeTab() === 'anc'}>
            <div class="panel-in">
              <AncTab
                mode={ancMode()}
                loading={ancLoading()}
                level={ancLevel()}
                levelMax={levelMax()}
                strengthModes={strengthModes()}
                supportedModes={supportedAncModes()}
                gameMode={gameMode()}
                showGameMode={true}
                onMode={handleAnc}
                onLevel={handleLevel}
                onLevelCommit={handleLevelCommit}
                onGameMode={handleGameMode}
              />
            </div>
          </Show>

          <Show when={activeTab() === 'eq'}>
            <div class="panel-in">
              <EqTab preset={eqPreset()} labels={modelInfo()?.eq_labels ?? null} onPreset={handleEqPreset} />
            </div>
          </Show>

          <Show when={activeTab() === 'settings'}>
            <div class="panel-in">
              <SettingsTab initialUpdateVersion={updateVersion()} onUpdateInstalled={() => setUpdateVersion(null)} />
            </div>
          </Show>
        </div>
      </div>
    </div>
  );
}
