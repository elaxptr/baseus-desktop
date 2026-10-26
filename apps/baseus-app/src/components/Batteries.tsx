import { Show } from 'solid-js';
import Zap from 'lucide-solid/icons/zap';
import type { BatteryState, CaseState, WearState } from '../lib/tauri';

interface Props {
  battery: BatteryState | null;
  case: CaseState | null;
  wear: WearState | null;
}

const LOW = 20;

export default function Batteries(props: Props) {
  return (
    <div class="batteries">
      <Battery
        label="Left"
        pct={props.battery?.left_pct ?? 0}
        charging={props.battery?.left_charging ?? false}
        inEar={props.wear ? props.wear.left_in_ear : null}
      />
      <Battery
        label="Right"
        pct={props.battery?.right_pct ?? 0}
        charging={props.battery?.right_charging ?? false}
        inEar={props.wear ? props.wear.right_in_ear : null}
      />
      <Battery label="Case" pct={props.case?.case_pct ?? 0} charging={props.case?.case_charging ?? false} inEar={null} />
    </div>
  );
}

/// 0 % reads as unknown (a bud the earbuds can't see), as in the low-battery alerts.
function Battery(p: { label: string; pct: number; charging: boolean; inEar: boolean | null }) {
  const known = () => p.pct > 0;
  return (
    <div class="card battery">
      <div class="battery-head">
        {p.label}
        <Show when={p.charging}>
          <Zap size={12} class="charging" aria-label="Charging" />
        </Show>
        <span class="spacer" />
        <Show when={p.inEar !== null}>
          <span class="wear" classList={{ in: p.inEar === true }} title={p.inEar ? 'In ear' : 'Not in ear'} />
        </Show>
      </div>
      <div class="battery-value num" classList={{ unknown: !known() }}>
        {known() ? p.pct : '–'}
        <Show when={known()}>
          <small>%</small>
        </Show>
      </div>
      <div class="meter" classList={{ low: known() && p.pct < LOW }}>
        <span style={{ width: `${p.pct}%` }} />
      </div>
    </div>
  );
}
