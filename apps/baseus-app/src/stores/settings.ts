import { createSignal } from 'solid-js';
import { getSettings, setSettings, type Settings } from '../lib/tauri';

const [settings, setSettingsSignal] = createSignal<Settings>({
  launch_at_login: true,
  low_battery_alerts: true,
  show_session_timer: true,
});

export async function loadSettings() {
  const s = await getSettings();
  setSettingsSignal(s);
}

export function getSettingsStore(): Settings {
  return settings();
}

export async function updateSetting<K extends keyof Settings>(
  key: K,
  value: Settings[K],
) {
  const next: Settings = { ...settings(), [key]: value };
  setSettingsSignal(next);
  await setSettings(next);
}
