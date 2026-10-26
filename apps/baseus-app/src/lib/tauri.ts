import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';

export interface BatteryState {
  left_pct: number;
  right_pct: number;
  left_charging: boolean;
  right_charging: boolean;
}

export interface CaseState {
  case_pct: number;
  case_charging: boolean;
}

export interface WearState {
  left_in_ear: boolean;
  right_in_ear: boolean;
}

export type AncMode = 'off' | 'anc' | 'transparency';

export type EqPreset = 'balanced' | 'bass_boost' | 'voice' | 'clear';

export interface ModelInfo {
  name: string;
  /** Discrete ANC strength steps (BP1 Ultra: 5), or null for the continuous 1–10 slider. */
  anc_levels: number | null;
  /** Noise-control modes that take a strength value. */
  strength_modes: AncMode[];
  find_earbud: boolean;
  /** Device-side names for the Balanced / Bass Boost / Voice / Clear presets. */
  eq_labels: [string, string, string, string] | null;
  /** Whether eq_preset_update events reflect the device's real preset. */
  eq_state: boolean;
}

export type DeviceEvent =
  | { type: 'battery_update'; data: BatteryState }
  | { type: 'case_update'; data: CaseState }
  | { type: 'anc_mode_update'; data: AncMode }
  | { type: 'anc_level_update'; data: number }
  | { type: 'game_mode_update'; data: boolean }
  | { type: 'wear_update'; data: WearState }
  | { type: 'eq_preset_update'; data: EqPreset }
  | { type: 'connected' }
  | { type: 'disconnected' };

export type ConnectionState = 'connecting' | 'connected' | 'disconnected';

export function onDeviceEvent(cb: (e: DeviceEvent) => void): Promise<UnlistenFn> {
  return listen<DeviceEvent>('device-event', (event) => cb(event.payload));
}

export function onConnectionState(cb: (s: ConnectionState) => void): Promise<UnlistenFn> {
  return listen<ConnectionState>('connection-state', (event) => cb(event.payload));
}

export function onModelInfo(cb: (info: ModelInfo) => void): Promise<UnlistenFn> {
  return listen<ModelInfo>('model-info', (event) => cb(event.payload));
}

/** Why the last connection attempt failed; null once connected. */
export interface ConnectionIssue {
  /** adapter: Bluetooth off/missing · not_found: nothing supported nearby ·
   *  connect_failed: earbuds found, control channel not opened. */
  kind: 'adapter' | 'not_found' | 'connect_failed';
  detail: string;
  /** Nearby devices that look like Baseus models this app doesn't support. */
  unsupported: string[];
  supported: string[];
}

export function onConnectionIssue(cb: (issue: ConnectionIssue | null) => void): Promise<UnlistenFn> {
  return listen<ConnectionIssue | null>('connection-issue', (event) => cb(event.payload));
}

export interface DeviceSnapshot {
  state: ConnectionState;
  model: ModelInfo | null;
  /** Latest device event of each kind since the connection started. */
  events: DeviceEvent[];
  issue: ConnectionIssue | null;
}

/** Connection state and model published before this window subscribed to the events. */
export function getDeviceSnapshot(): Promise<DeviceSnapshot> {
  return invoke('get_device_snapshot');
}

export interface Settings {
  launch_at_login: boolean;
  low_battery_alerts: boolean;
  show_session_timer: boolean;
}

export function setAncMode(mode: AncMode, level?: number): Promise<void> {
  return invoke('set_anc_mode', { mode, level });
}

export function setGameMode(enabled: boolean): Promise<void> {
  return invoke('set_game_mode', { enabled });
}

export function findEarbud(side: 'left' | 'right'): Promise<void> {
  return invoke('find_earbud', { side });
}

export function getSettings(): Promise<Settings> {
  return invoke('get_settings');
}

export function setSettings(settings: Settings): Promise<void> {
  return invoke('set_settings', { settings });
}

export function setEqPreset(preset: EqPreset): Promise<void> {
  const map: Record<EqPreset, number> = { balanced: 0, bass_boost: 1, voice: 2, clear: 3 };
  return invoke('set_eq_preset', { preset: map[preset] });
}

export function getSupportedAncModes(modelName: string): Promise<AncMode[]> {
  return invoke('get_supported_anc_modes', { modelName });
}

/** A command the earbuds didn't take: the write itself failed. */
export interface CommandError {
  command: 'anc' | 'eq' | 'game' | 'find';
  message: string;
}

export function onCommandError(cb: (e: CommandError) => void): Promise<UnlistenFn> {
  return listen<CommandError>('command-error', (event) => cb(event.payload));
}

export function onUpdateAvailable(cb: (version: string) => void): Promise<UnlistenFn> {
  return listen<string>('update-available', (event) => cb(event.payload));
}

export function checkForUpdate(): Promise<string | null> {
  return invoke('check_for_update');
}

export function installUpdate(): Promise<void> {
  return invoke('install_update');
}
