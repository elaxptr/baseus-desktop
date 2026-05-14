import { listen, type UnlistenFn } from '@tauri-apps/api/event';

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
