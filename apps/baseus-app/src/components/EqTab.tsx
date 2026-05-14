import type { EqPreset } from '../lib/tauri';

const PRESETS: { id: EqPreset; name: string; bars: number[] }[] = [
  { id: 'balanced',  name: 'Balanced',   bars: [50, 55, 60, 55, 50] },
  { id: 'bass_boost', name: 'Bass Boost', bars: [100, 85, 55, 40, 35] },
  { id: 'voice',     name: 'Voice',      bars: [30, 60, 100, 80, 40] },
  { id: 'clear',     name: 'Clear',      bars: [35, 45, 55, 75, 100] },
];

interface Props {
  preset: EqPreset;
  onPreset: (p: EqPreset) => void;
}

export default function EqTab(props: Props) {
  return (
    <div>
      <div style={labelStyle}>Sound Presets <Divider /></div>
      <div style={{ display: 'grid', 'grid-template-columns': '1fr 1fr', gap: '8px' }}>
        {PRESETS.map(({ id, name, bars }) => {
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
