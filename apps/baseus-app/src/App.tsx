import { createMemo, createSignal, onCleanup, onMount, Show } from 'solid-js';
import Sidebar, { type Tab } from './components/Sidebar';
import HomeTab from './components/HomeTab';
import AncTab from './components/AncTab';
import EqTab from './components/EqTab';
import SettingsTab from './components/SettingsTab';
import ConnectionHelp from './components/ConnectionHelp';
import {
  getDeviceSnapshot,
  onDeviceEvent,
  onConnectionState,
  onConnectionIssue,
  onModelInfo,
  onCommandError,
  onUpdateAvailable,
  setAncMode,
  setEqPreset,
  setGameMode,
  type AncMode,
  type CommandError,
  type ConnectionIssue,
  type DeviceEvent,
  type EqPreset,
  type ModelInfo,
  type WearState,
} from './lib/tauri';
import { pushLeft, pushRight, pushCase, left, right, caseData } from './stores/batteryHistory';
import { loadSettings, getSettingsStore } from './stores/settings';
import { startTimer, stopTimer, useElapsed } from './lib/timer';
import { showError, toastMessage } from './lib/toast';

type ConnStatus = 'connected' | 'connecting' | 'disconnected';

const COMMAND_VERB: Record<CommandError['command'], string> = {
  anc: 'change noise control',
  eq: 'change the sound preset',
  game: 'switch game mode',
  find: 'play the sound',
};

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
  const [issue, setIssue] = createSignal<ConnectionIssue | null>(null);

  const connected = () => status() === 'connected';
  const connectedModelName = createMemo(() => modelInfo()?.name ?? 'Baseus Desktop');
  /// Undo the optimistic UI change of the latest command of each kind, should it fail.
  const reverts: Partial<Record<CommandError['command'], () => void>> = {};
  const failed = (kind: CommandError['command']) => (e: unknown) => {
    reverts[kind]?.();
    showError(e);
  };
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
    let issueSeen = false;
    unlisteners.push(
      await onConnectionIssue((i) => {
        issueSeen = true;
        setIssue(i);
      }),
    );

    // The backend may have connected before these listeners existed (the webview loads
    // in the background); catch up on what was already published. Model first: event
    // handling depends on its capabilities. Skip event kinds a listener already delivered,
    // which are newer than the snapshot's.
    const snapshot = await getDeviceSnapshot().catch(() => null);
    if (snapshot?.model && modelInfo() === null) applyModel(snapshot.model);
    if (snapshot && status() === 'connecting') applyStatus(snapshot.state);
    if (snapshot && !issueSeen) setIssue(snapshot.issue);
    snapshot?.events.filter((e) => !seen.has(e.type)).forEach(handleDeviceEvent);

    onUpdateAvailable((version) => setUpdateVersion(version))
      .then((fn) => unlisteners.push(fn));
    // The command was queued but the write to the earbuds failed.
    unlisteners.push(
      await onCommandError((e) => failed(e.command)(`Couldn't ${COMMAND_VERB[e.command]}: ${e.message}`)),
    );
  });

  async function handleAnc(mode: AncMode) {
    if (ancMode() === mode) return;
    setAncLoading(mode);
    reverts.anc = () => setAncLoading(null);
    await setAncMode(mode, mode === 'off' ? undefined : levelByte(ancLevel())).catch(failed('anc'));
  }

  async function handleGameMode(on: boolean) {
    setGameModeSignal(on);
    reverts.game = () => setGameModeSignal(!on);
    await setGameMode(on).catch(failed('game'));
  }

  async function handleEqPreset(preset: EqPreset) {
    const previous = eqPreset();
    if (previous === preset) return;
    setEqPresetSignal(preset);
    reverts.eq = () => setEqPresetSignal(previous);
    await setEqPreset(preset).catch(failed('eq'));
  }

  function handleLevel(v: number) {
    setAncLevel(v);
  }

  async function handleLevelCommit(v: number) {
    setAncLevel(v);
    const mode = ancMode();
    if (strengthModes().includes(mode)) {
      reverts.anc = undefined;
      await setAncMode(mode, levelByte(v)).catch(failed('anc'));
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
        <div
          style={{
            flex: '1',
            'font-size': '12px',
            'font-weight': '600',
            color: '#444',
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
              <Show when={!connected()}>
                <ConnectionHelp connecting={status() === 'connecting'} issue={issue()} />
              </Show>
              {/* Battery and wear readings belong to a live connection; hide them rather than show 0 %. */}
              <Show when={connected()}>
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
              </Show>
            </div>
          </Show>

          <Show when={activeTab() === 'anc'}>
            <div class="panel-in">
              <Offline when={!connected()} />
              <div class="device-controls" inert={!connected()} classList={{ offline: !connected() }}>
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
            </div>
          </Show>

          <Show when={activeTab() === 'eq'}>
            <div class="panel-in">
              <Offline when={!connected()} />
              <div class="device-controls" inert={!connected()} classList={{ offline: !connected() }}>
                <EqTab preset={eqPreset()} labels={modelInfo()?.eq_labels ?? null} onPreset={handleEqPreset} />
              </div>
            </div>
          </Show>

          <Show when={activeTab() === 'settings'}>
            <div class="panel-in">
              <SettingsTab initialUpdateVersion={updateVersion()} onUpdateInstalled={() => setUpdateVersion(null)} />
            </div>
          </Show>
        </div>
      </div>

      <Show when={toastMessage()}>
        {(message) => (
          <div class="toast" role="alert">
            {message()}
          </div>
        )}
      </Show>
    </div>
  );
}

function Offline(p: { when: boolean }) {
  return (
    <Show when={p.when}>
      <div class="offline-note">Connect your earbuds to change these settings.</div>
    </Show>
  );
}
