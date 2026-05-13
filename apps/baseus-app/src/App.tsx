import { createSignal, onCleanup, onMount } from 'solid-js';
import BatteryCard from './components/BatteryCard';
import ConnectionCard from './components/ConnectionCard';
import { BatteryState, connectDevice, onDeviceEvent } from './lib/tauri';

// The Bluetooth address of the BP1 Pro ANC.
// Set VITE_BT_ADDR in apps/baseus-app/.env.local (hex, no colons).
// e.g. VITE_BT_ADDR=AABBCCDDEEFF
const DEVICE_ADDR = BigInt('0x' + (import.meta.env.VITE_BT_ADDR ?? '000000000000'));

type ConnStatus = 'connected' | 'connecting' | 'disconnected';

export default function App() {
  const [status, setStatus] = createSignal<ConnStatus>('connecting');
  const [battery, setBattery] = createSignal<BatteryState | null>(null);
  const [lastUpd, setLastUpd] = createSignal<string | null>(null);

  onMount(() => {
    let unlistenFn: (() => void) | undefined;
    onCleanup(() => unlistenFn?.());  // registered synchronously

    onDeviceEvent((e) => {
      if (e.type === 'battery_update') {
        const { type: _t, ...state } = e;
        setBattery(state);
        setStatus('connected');
        setLastUpd(new Date().toLocaleTimeString());
      } else if (e.type === 'connected') {
        setStatus('connected');
      } else if (e.type === 'disconnected') {
        setStatus('disconnected');
      }
    }).then((fn) => {
      unlistenFn = fn;
    });

    if (DEVICE_ADDR !== 0n) {
      connectDevice(DEVICE_ADDR).catch((err) => {
        console.error('connect failed:', err);
        setStatus('disconnected');
      });
    }
  });

  return (
    <div class="min-h-screen bg-neutral-950 flex flex-col items-center justify-center gap-6 p-6">
      <h1 class="text-lg font-semibold text-neutral-200 tracking-tight">Baseus Desktop</h1>
      <ConnectionCard status={status()} lastUpdated={lastUpd()} />
      <div class="flex gap-4 flex-wrap justify-center">
        <BatteryCard label="Left" pct={battery()?.left_pct ?? 0} charging={battery()?.left_charging ?? false} />
        <BatteryCard label="Right" pct={battery()?.right_pct ?? 0} charging={battery()?.right_charging ?? false} />
        <BatteryCard label="Case" pct={battery()?.case_pct ?? 0} charging={battery()?.case_charging ?? false} />
      </div>
      <p class="text-xs text-neutral-600">
        {status() === 'disconnected' ? 'Open the case to reconnect.' : 'Showing live battery readings.'}
      </p>
    </div>
  );
}
