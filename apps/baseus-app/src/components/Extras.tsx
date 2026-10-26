import { Show } from 'solid-js';
import Gamepad2 from 'lucide-solid/icons/gamepad-2';
import Volume2 from 'lucide-solid/icons/volume-2';
import Switch from './Switch';
import { findEarbud } from '../lib/tauri';
import { showError } from '../lib/toast';

interface Props {
  lowLatency: boolean;
  onLowLatency: (on: boolean) => void;
  showFind: boolean; // false for models whose find-my-earbuds command is unknown
}

export default function Extras(props: Props) {
  return (
    <section class="card rows">
      <div class="row">
        <div class="row-icon" classList={{ on: props.lowLatency }}>
          <Gamepad2 size={18} />
        </div>
        <div class="row-text">
          <div class="row-title">Low latency</div>
          <div class="row-desc">Less audio delay for games and video</div>
        </div>
        <Switch on={props.lowLatency} label="Low latency" onChange={props.onLowLatency} />
      </div>

      <Show when={props.showFind}>
        <div class="row">
          <div class="row-icon">
            <Volume2 size={18} />
          </div>
          <div class="row-text">
            <div class="row-title">Find earbuds</div>
            <div class="row-desc">Plays a tone for 5 seconds</div>
          </div>
          <button class="btn" onClick={() => findEarbud('left').catch(showError)}>Left</button>
          <button class="btn" onClick={() => findEarbud('right').catch(showError)}>Right</button>
        </div>
      </Show>
    </section>
  );
}
