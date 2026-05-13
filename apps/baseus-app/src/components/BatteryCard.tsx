import { Component } from 'solid-js';

interface Props {
  label: string;
  pct: number;
  charging: boolean;
}

const BatteryCard: Component<Props> = (props) => {
  const RADIUS = 38;
  const CIRC = 2 * Math.PI * RADIUS;
  const offset = () => CIRC - (Math.max(0, Math.min(100, props.pct)) / 100) * CIRC;
  const color = () => (props.pct > 20 ? '#22c55e' : '#ef4444');

  return (
    <div class="flex flex-col items-center gap-3 rounded-2xl bg-neutral-900 p-6 w-40">
      <div class="relative w-24 h-24">
        <svg class="absolute inset-0 -rotate-90" viewBox="0 0 88 88">
          <circle cx="44" cy="44" r={RADIUS} fill="none" stroke="#262626" stroke-width="8" />
          <circle
            cx="44"
            cy="44"
            r={RADIUS}
            fill="none"
            stroke={color()}
            stroke-width="8"
            stroke-dasharray={String(CIRC)}
            stroke-dashoffset={String(offset())}
            stroke-linecap="round"
            style="transition: stroke-dashoffset 0.5s ease, stroke 0.3s ease"
          />
        </svg>
        <span class="absolute inset-0 flex items-center justify-center text-2xl font-bold tabular-nums">
          {props.pct}
        </span>
      </div>
      <span class="text-sm text-neutral-400 font-medium">{props.label}</span>
      {props.charging && (
        <span class="text-xs text-yellow-400 font-semibold tracking-wide">CHARGING</span>
      )}
    </div>
  );
};

export default BatteryCard;
