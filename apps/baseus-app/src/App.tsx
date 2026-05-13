import { createSignal, onCleanup, onMount } from 'solid-js';
import BatteryCard from './components/BatteryCard';
import ConnectionCard from './components/ConnectionCard';
import { BatteryState, onConnectionState, onDeviceEvent } from './lib/tauri';

type ConnStatus = 'connected' | 'connecting' | 'disconnected';

export default function App() {
  const [status, setStatus] = createSignal<ConnStatus>('connecting');
  const [battery, setBattery] = createSignal<BatteryState | null>(null);
  const [lastUpd, setLastUpd] = createSignal<string | null>(null);

  onMount(() => {
    const unlisteners: Array<() => void> = [];
    onCleanup(() => unlisteners.forEach((fn) => fn()));

    onDeviceEvent((e) => {
      if (e.type === 'battery_update') {
        setBattery(e.data);
        setLastUpd(new Date().toLocaleTimeString());
      }
    }).then((fn) => unlisteners.push(fn));

    onConnectionState((s) => {
      setStatus(s);
    }).then((fn) => unlisteners.push(fn));
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
