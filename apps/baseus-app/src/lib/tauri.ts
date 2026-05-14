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

export type DeviceEvent =
  | { type: 'battery_update'; data: BatteryState }
  | { type: 'case_update'; data: CaseState }
  | { type: 'anc_mode_update'; data: 'off' | 'anc' | 'transparency' }
  | { type: 'connected' }
  | { type: 'disconnected' };

export type ConnectionState = 'connecting' | 'connected' | 'disconnected';

export function onDeviceEvent(cb: (e: DeviceEvent) => void): Promise<UnlistenFn> {
  return listen<DeviceEvent>('device-event', (event) => cb(event.payload));
}

export function onConnectionState(cb: (s: ConnectionState) => void): Promise<UnlistenFn> {
  return listen<ConnectionState>('connection-state', (event) => cb(event.payload));
}

export interface Settings {
  launch_at_login: boolean;
  low_battery_alerts: boolean;
}

export function setAncMode(mode: 'off' | 'anc' | 'transparency'): Promise<void> {
  return invoke('set_anc_mode', { mode });
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
