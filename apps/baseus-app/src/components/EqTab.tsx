import { Show } from 'solid-js';
import type { BassBoost, EqPreset } from '../lib/tauri';
import { Segmented, SwitchRow } from './Controls';

const PRESETS: { id: EqPreset; name: string; bars: number[] }[] = [
  { id: 'balanced',  name: 'Balanced',   bars: [50, 55, 60, 55, 50] },
  { id: 'bass_boost', name: 'Bass Boost', bars: [100, 85, 55, 40, 35] },
  { id: 'voice',     name: 'Voice',      bars: [30, 60, 100, 80, 40] },
  { id: 'clear',     name: 'Clear',      bars: [35, 45, 55, 75, 100] },
];

interface Props {
  preset: EqPreset;
  /** Model-specific preset names, in PRESETS order; null keeps the generic names. */
  labels: [string, string, string, string] | null;
  onPreset: (p: EqPreset) => void;
  /** Bass-boost level count, or null to hide the control. */
  bassBoostLevels: number | null;
  bassBoost: BassBoost;
  onBassBoost: (b: BassBoost) => void;
  /** Whether the model has spatial audio. */
  showSpatialAudio: boolean;
  spatialAudio: boolean;
  onSpatialAudio: (on: boolean) => void;
}

export default function EqTab(props: Props) {
  return (
    <div>
      <div style={labelStyle}>Sound Presets <Divider /></div>
      <div style={{ display: 'grid', 'grid-template-columns': '1fr 1fr', gap: '8px' }}>
        {PRESETS.map(({ id, name: genericName, bars }, i) => {
          const name = props.labels?.[i] ?? genericName;
          const active = () => props.preset === id;
          return (
            <div
              onClick={() => props.onPreset(id)}
              style={{
                background: active() ? 'rgba(99,102,241,0.12)' : '#111113',
                border: active() ? '1px solid rgba(99,102,241,0.5)' : '1px solid #1a1a1e',
                'border-radius': '12px',
                padding: '12px',
                cursor: 'pointer',
                transition: 'background 0.15s, border-color 0.15s',
              }}
            >
              <div style={{ 'font-size': '12px', 'font-weight': '600', color: active() ? '#a5b4fc' : '#888', 'margin-bottom': '8px' }}>
                {name}
              </div>
              <div style={{ display: 'flex', 'align-items': 'flex-end', gap: '3px', height: '28px' }}>
                {bars.map((h) => (
                  <div
                    style={{
                      flex: '1',
                      height: `${h}%`,
                      background: active() ? '#818cf8' : '#333',
                      'border-radius': '2px',
                      transition: 'background 0.15s',
                    }}
                  />
                ))}
              </div>
            </div>
          );
        })}
      </div>

      <Show when={props.bassBoostLevels}>
        {(levels) => (
          <>
            <div style={{ ...labelStyle, 'margin-top': '16px' }}>Bass Boost <Divider /></div>
            <Segmented
              options={[
                { value: 0, label: 'Off' },
                ...Array.from({ length: levels() }, (_, i) => ({ value: i + 1, label: `${i + 1}` })),
              ]}
              value={props.bassBoost.enabled ? props.bassBoost.level : 0}
              // Off keeps the level, so turning boost back on restores it.
              onChange={(v) =>
                props.onBassBoost(v === 0 ? { ...props.bassBoost, enabled: false } : { enabled: true, level: v })
              }
            />
          </>
        )}
      </Show>

      <Show when={props.showSpatialAudio}>
        <div style={{ ...labelStyle, 'margin-top': '16px' }}>Spatial Audio <Divider /></div>
        <SwitchRow
          icon="🎧"
          label="Spatial Audio"
          desc="Wider, surround-like soundstage"
          value={props.spatialAudio}
          onChange={props.onSpatialAudio}
        />
      </Show>
    </div>
  );
}

function Divider() {
  return <div style={{ flex: '1', height: '1px', background: '#161618' }} />;
}

const labelStyle = {
  'font-size': '9px',
  'font-weight': '700',
  color: '#333',
  'letter-spacing': '0.12em',
  'text-transform': 'uppercase' as const,
  display: 'flex',
  'align-items': 'center',
  gap: '8px',
  'margin-bottom': '8px',
};
