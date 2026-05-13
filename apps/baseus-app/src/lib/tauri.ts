import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export interface BatteryState {
  left_pct: number;
  right_pct: number;
  case_pct: number;
  left_charging: boolean;
  right_charging: boolean;
  case_charging: boolean;
}

export type DeviceEvent =
  | ({ type: 'battery_update' } & BatteryState)
  | { type: 'connected' }
  | { type: 'disconnected' };

export function connectDevice(addr: bigint): Promise<void> {
  return invoke('connect', { addr });
}

export function onDeviceEvent(cb: (e: DeviceEvent) => void): Promise<UnlistenFn> {
  return listen<DeviceEvent>('device-event', (event) => cb(event.payload));
}
