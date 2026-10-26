import { createMemo, createSignal, onCleanup, onMount, Show } from 'solid-js';
import Header from './components/Header';
import Batteries from './components/Batteries';
import NoiseControl from './components/NoiseControl';
import SoundPresets from './components/SoundPresets';
import Extras from './components/Extras';
import Settings from './components/Settings';
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
  type BatteryState,
  type CaseState,
  type CommandError,
  type ConnectionIssue,
  type DeviceEvent,
  type EqPreset,
  type ModelInfo,
  type WearState,
} from './lib/tauri';
import { loadSettings, getSettingsStore } from './stores/settings';
import { formatElapsed, startTimer, stopTimer, useElapsed } from './lib/timer';
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
  const [settingsOpen, setSettingsOpen] = createSignal(false);
  const [battery, setBattery] = createSignal<BatteryState | null>(null);
  const [caseState, setCaseState] = createSignal<CaseState | null>(null);
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
        setBattery(e.data);
      } else if (e.type === 'case_update') {
        setCaseState(e.data);
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
      // Readings from the previous connection don't belong to these earbuds.
      setBattery(null);
      setCaseState(null);
      setWear(null);
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

  const statusText = () => {
    if (!connected()) return 'Not connected';
    const elapsed = useElapsed()();
    return getSettingsStore().show_session_timer && elapsed > 0
      ? `Connected · ${formatElapsed(elapsed)}`
      : 'Connected';
  };

  return (
    <div class="app">
      <Header
        title={connectedModelName()}
        status={statusText()}
        connected={connected()}
        settingsOpen={settingsOpen()}
        updateAvailable={updateVersion() !== null}
        onSettings={() => setSettingsOpen(true)}
        onBack={() => setSettingsOpen(false)}
      />

      <main class="content">
        <Show
          when={settingsOpen()}
          fallback={
            <div class="view-in content-stack">
              <Show
                when={connected()}
                fallback={<ConnectionHelp issue={issue()} />}
              >
                <Batteries battery={battery()} case={caseState()} wear={wear()} />
              </Show>

              <div class="device-controls" inert={!connected()} classList={{ offline: !connected() }}>
                <NoiseControl
                  mode={ancMode()}
                  pending={ancLoading()}
                  modes={supportedAncModes()}
                  level={ancLevel()}
                  levelMax={levelMax()}
                  showStrength={strengthModes().includes(ancMode())}
                  onMode={handleAnc}
                  onLevel={handleLevel}
                  onLevelCommit={handleLevelCommit}
                />
                <SoundPresets preset={eqPreset()} labels={modelInfo()?.eq_labels ?? null} onPreset={handleEqPreset} />
                <Extras
                  lowLatency={gameMode()}
                  onLowLatency={handleGameMode}
                  showFind={modelInfo()?.find_earbud ?? true}
                />
              </div>
            </div>
          }
        >
          <div class="view-in content-stack">
            <Settings initialUpdateVersion={updateVersion()} onUpdateInstalled={() => setUpdateVersion(null)} />
          </div>
        </Show>
      </main>

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
