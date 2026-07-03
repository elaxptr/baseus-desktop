import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';

export interface BatteryState {
  left_pct: number;
  right_pct: number;
  left_charging: boolean;
  right_charging: boolean;
}

export interface HeadphoneBattery {
  pct: number;
  charging: boolean;
}

export interface CaseState {
  case_pct: number;
  case_charging: boolean;
}

export interface WearState {
  left_in_ear: boolean;
  right_in_ear: boolean;
}

export type AncMode =
  | 'off'
  | 'anc'
  | 'transparency'
  // Inspire XH1 adaptive modes — APK-extracted, unverified wire format
  | 'adaptive_self'
  | 'adaptive_indoor'
  | 'adaptive_outdoor'
  | 'adaptive_commute';

export type EqPreset = 'balanced' | 'bass_boost' | 'voice' | 'clear';

export type ModelStatus = 'verified' | 'experimental';

export interface ModelInfo {
  name: string;
  status: ModelStatus;
}

export type DeviceEvent =
  | { type: 'battery_update'; data: BatteryState }
  | { type: 'headphone_battery_update'; data: HeadphoneBattery }
  | { type: 'case_update'; data: CaseState }
  | { type: 'anc_mode_update'; data: AncMode }
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

export function onUpdateAvailable(cb: (version: string) => void): Promise<UnlistenFn> {
  return listen<string>('update-available', (event) => cb(event.payload));
}

export function checkForUpdate(): Promise<string | null> {
  return invoke('check_for_update');
}

export function installUpdate(): Promise<void> {
  return invoke('install_update');
}
