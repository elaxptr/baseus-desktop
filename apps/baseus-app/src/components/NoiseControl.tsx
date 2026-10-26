import { For, Show, type JSX } from 'solid-js';
import Ear from 'lucide-solid/icons/ear';
import Headphones from 'lucide-solid/icons/headphones';
import VolumeX from 'lucide-solid/icons/volume-x';
import type { AncMode } from '../lib/tauri';

interface Props {
  mode: AncMode;
  pending: AncMode | null;
  modes: AncMode[];
  level: number; // 1–levelMax
  levelMax: number; // 10 (continuous) or the model's discrete step count
  showStrength: boolean;
  onMode: (mode: AncMode) => void;
  onLevel: (level: number) => void;
  onLevelCommit: (level: number) => void;
}

const MODE_META: Record<AncMode, { name: string; icon: () => JSX.Element }> = {
  off: { name: 'Off', icon: () => <VolumeX size={20} /> },
  anc: { name: 'Noise cancelling', icon: () => <Headphones size={20} /> },
  transparency: { name: 'Transparency', icon: () => <Ear size={20} /> },
};

export default function NoiseControl(props: Props) {
  const value = (e: Event) => Number((e.target as HTMLInputElement).value);
  const fill = () => `${((props.level - 1) / Math.max(props.levelMax - 1, 1)) * 100}%`;

  return (
    <section>
      <h2 class="section-title">Noise control</h2>
      <div class="card noise">
        <div class="segmented" role="radiogroup" aria-label="Noise control">
          <For each={props.modes}>
            {(mode) => (
              <button
                class="segment"
                classList={{ active: props.mode === mode, pending: props.pending === mode }}
                role="radio"
                aria-checked={props.mode === mode}
                onClick={() => props.onMode(mode)}
              >
                {MODE_META[mode].icon()}
                {MODE_META[mode].name}
              </button>
            )}
          </For>
        </div>

        <Show when={props.showStrength}>
          <div class="strength">
            <div class="strength-row">
              Strength
              <span class="num">
                {props.level} / {props.levelMax}
              </span>
            </div>
            <input
              type="range"
              min="1"
              max={props.levelMax}
              value={props.level}
              style={{ '--fill': fill() }}
              aria-label="Strength"
              onInput={(e) => props.onLevel(value(e))}
              onChange={(e) => props.onLevelCommit(value(e))}
            />
          </div>
        </Show>
      </div>
    </section>
  );
}
