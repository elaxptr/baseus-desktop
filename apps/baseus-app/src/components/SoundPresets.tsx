import { For, Show } from 'solid-js';
import AudioLines from 'lucide-solid/icons/audio-lines';
import Orbit from 'lucide-solid/icons/orbit';
import Switch from './Switch';
import type { BassBoost, EqPreset } from '../lib/tauri';

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
  /** Bass-boost level count, or null when the model has none. */
  bassBoostLevels: number | null;
  bassBoost: BassBoost;
  onBassBoost: (next: BassBoost) => void;
  showSpatialAudio: boolean;
  spatialAudio: boolean;
  onSpatialAudio: (on: boolean) => void;
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

      <Show when={props.bassBoostLevels || props.showSpatialAudio}>
        <div class="card rows sound-extras">
          <Show when={props.bassBoostLevels}>
            {(levels) => (
              <div class="row">
                <div class="row-icon" classList={{ on: props.bassBoost.enabled }}>
                  <AudioLines size={18} />
                </div>
                <div class="row-text">
                  <div class="row-title">Bass boost</div>
                </div>
                <BassChoice levels={levels()} value={props.bassBoost} onChange={props.onBassBoost} />
              </div>
            )}
          </Show>
          <Show when={props.showSpatialAudio}>
            <div class="row">
              <div class="row-icon" classList={{ on: props.spatialAudio }}>
                <Orbit size={18} />
              </div>
              <div class="row-text">
                <div class="row-title">Spatial audio</div>
                <div class="row-desc">Wider, more open sound</div>
              </div>
              <Switch on={props.spatialAudio} label="Spatial audio" onChange={props.onSpatialAudio} />
            </div>
          </Show>
        </div>
      </Show>
    </section>
  );
}

/// Off plus one button per level. Off keeps the level, as the earbuds do, so turning
/// boost back on restores it.
function BassChoice(p: { levels: number; value: BassBoost; onChange: (next: BassBoost) => void }) {
  const levels = () => Array.from({ length: p.levels }, (_, i) => i + 1);
  return (
    <div class="choice" role="radiogroup" aria-label="Bass boost">
      <button
        classList={{ active: !p.value.enabled }}
        role="radio"
        aria-checked={!p.value.enabled}
        onClick={() => p.value.enabled && p.onChange({ ...p.value, enabled: false })}
      >
        Off
      </button>
      <For each={levels()}>
        {(level) => {
          const active = () => p.value.enabled && p.value.level === level;
          return (
            <button
              class="num"
              classList={{ active: active() }}
              role="radio"
              aria-checked={active()}
              onClick={() => !active() && p.onChange({ enabled: true, level })}
            >
              {level}
            </button>
          );
        }}
      </For>
    </div>
  );
}
