import type { AncMode } from '../lib/tauri';

interface Props {
  mode: AncMode;
  loading: AncMode | null;
  level: number; // 1–10
  onMode: (mode: AncMode) => void;
  onLevel: (level: number) => void;
}

const MODES: Array<{ mode: AncMode; icon: string; name: string; desc: string }> = [
  { mode: 'off',          icon: '🔇', name: 'Off',                     desc: 'Passthrough — no processing' },
  { mode: 'anc',          icon: '🎧', name: 'Active Noise Cancellation', desc: 'Blocks ambient sound' },
  { mode: 'transparency', icon: '🌬️', name: 'Transparency',             desc: 'Lets ambient sound in' },
];

export default function AncTab(props: Props) {
  const levelByte = (v: number) => Math.round(((v - 1) / 9) * (0xff - 0x10) + 0x10);

  function handleSlider(e: Event) {
    const v = Number((e.target as HTMLInputElement).value);
    props.onLevel(v);
  }

  return (
    <div>
      <div style={labelStyle}>Noise Control <Divider /></div>

      <div style={{ display: 'flex', 'flex-direction': 'column', gap: '8px', 'margin-bottom': '16px' }}>
        {MODES.map(({ mode, icon, name, desc }) => {
          const isActive = () => props.mode === mode;
          const isLoading = () => props.loading === mode;
          return (
            <button
              onClick={() => props.onMode(mode)}
              style={{
                background: isActive() ? 'rgba(99,102,241,0.08)' : '#111113',
                border: `1px solid ${isActive() ? 'rgba(99,102,241,0.4)' : '#1a1a1e'}`,
                'border-radius': '12px',
                padding: '14px 16px',
                display: 'flex',
                'align-items': 'center',
                gap: '12px',
                cursor: 'pointer',
                transition: 'border-color 0.12s, background 0.12s',
                animation: isLoading() ? 'pulse 0.8s ease-in-out infinite' : 'none',
                width: '100%',
                'text-align': 'left',
              }}
            >
              <span style={{ 'font-size': '20px', width: '28px', 'text-align': 'center' }}>{icon}</span>
              <div style={{ flex: '1' }}>
                <div style={{ 'font-size': '13px', 'font-weight': '600', color: isActive() ? '#c7d2fe' : '#aaa' }}>
                  {name}
                </div>
                <div style={{ 'font-size': '10px', color: '#444', 'margin-top': '2px' }}>{desc}</div>
              </div>
              {isActive() && (
                <div
                  style={{
                    width: '16px', height: '16px', 'border-radius': '50%',
                    background: '#6366f1', display: 'flex',
                    'align-items': 'center', 'justify-content': 'center',
                    'font-size': '9px', color: '#fff', 'flex-shrink': '0',
                  }}
                >
                  ✓
                </div>
              )}
            </button>
          );
        })}
      </div>

      {/* Level slider — only meaningful for ANC and Transparency */}
      {props.mode !== 'off' && (
        <>
          <div style={labelStyle}>Strength <Divider /></div>
          <div
            style={{
              background: '#111113',
              border: '1px solid #1a1a1e',
              'border-radius': '12px',
              padding: '14px 16px',
            }}
          >
            <div style={{ display: 'flex', 'justify-content': 'space-between', 'margin-bottom': '12px' }}>
              <span style={{ 'font-size': '12px', color: '#888' }}>Level</span>
              <span style={{ 'font-size': '12px', 'font-weight': '700', color: '#818cf8' }}>
                {props.level} / 10
              </span>
            </div>
            <input
              type="range" min="1" max="10" value={props.level}
              onInput={handleSlider}
              style={{
                width: '100%', height: '4px',
                'accent-color': '#6366f1',
                cursor: 'pointer',
              }}
            />
            <div style={{ display: 'flex', 'justify-content': 'space-between', 'margin-top': '6px', 'font-size': '9px', color: '#333' }}>
              <span>Low</span><span>High</span>
            </div>
          </div>
        </>
      )}
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
