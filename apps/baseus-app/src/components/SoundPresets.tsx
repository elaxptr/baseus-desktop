import { For } from 'solid-js';
import type { EqPreset } from '../lib/tauri';

const PRESETS: { id: EqPreset; name: string; bars: number[] }[] = [
  { id: 'balanced', name: 'Balanced', bars: [50, 55, 60, 55, 50] },
  { id: 'bass_boost', name: 'Bass Boost', bars: [100, 85, 55, 40, 35] },
  { id: 'voice', name: 'Voice', bars: [30, 60, 100, 80, 40] },
  { id: 'clear', name: 'Clear', bars: [35, 45, 55, 75, 100] },
];

interface Props {
  preset: EqPreset;
  /** Model-specific preset names, in PRESETS order; null keeps the generic names. */
  labels: [string, string, string, string] | null;
  onPreset: (p: EqPreset) => void;
}

export default function SoundPresets(props: Props) {
  return (
    <section>
      <h2 class="section-title">Sound</h2>
      <div class="presets" role="radiogroup" aria-label="Sound preset">
        <For each={PRESETS}>
          {({ id, name, bars }, i) => (
            <button
              class="card preset"
              classList={{ active: props.preset === id }}
              role="radio"
              aria-checked={props.preset === id}
              onClick={() => props.onPreset(id)}
            >
              <div class="preset-bars">
                <For each={bars}>{(h) => <span style={{ height: `${h}%` }} />}</For>
              </div>
              <div class="preset-name">{props.labels?.[i()] ?? name}</div>
            </button>
          )}
        </For>
      </div>
    </section>
  );
}
