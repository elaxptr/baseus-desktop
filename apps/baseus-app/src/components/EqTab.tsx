const PRESETS = [
  { id: 'balanced',  name: 'Balanced',   bars: [50, 55, 60, 55, 50] },
  { id: 'bass',      name: 'Bass Boost', bars: [100, 85, 55, 40, 35] },
  { id: 'voice',     name: 'Voice',      bars: [30, 60, 100, 80, 40] },
  { id: 'clear',     name: 'Clear',      bars: [35, 45, 55, 75, 100] },
];

export default function EqTab() {
  return (
    <div>
      <div style={labelStyle}>Sound Presets <Divider /></div>
      <div style={{ display: 'grid', 'grid-template-columns': '1fr 1fr', gap: '8px', 'margin-bottom': '14px' }}>
        {PRESETS.map(({ name, bars }) => (
          <div
            style={{
              background: '#111113',
              border: '1px solid #1a1a1e',
              'border-radius': '12px',
              padding: '12px',
              opacity: '0.45',
              cursor: 'not-allowed',
            }}
          >
            <div style={{ 'font-size': '12px', 'font-weight': '600', color: '#888', 'margin-bottom': '8px' }}>
              {name}
            </div>
            <div style={{ display: 'flex', 'align-items': 'flex-end', gap: '3px', height: '28px' }}>
              {bars.map((h) => (
                <div
                  style={{
                    flex: '1',
                    height: `${h}%`,
                    background: '#333',
                    'border-radius': '2px',
                  }}
                />
              ))}
            </div>
          </div>
        ))}
      </div>
      <ReNotice feature="EQ presets" />
    </div>
  );
}

function Divider() {
  return <div style={{ flex: '1', height: '1px', background: '#161618' }} />;
}

function ReNotice(p: { feature: string }) {
  return (
    <div
      style={{
        background: 'rgba(99,102,241,0.06)',
        border: '1px solid rgba(99,102,241,0.15)',
        'border-radius': '10px',
        padding: '12px 14px',
        'font-size': '11px',
        color: '#555',
        'line-height': '1.5',
      }}
    >
      <span style={{ color: '#818cf8', 'font-weight': '600' }}>Protocol RE needed — </span>
      {p.feature} require capturing the BLE write bytes from the Android app via Frida. Once captured, this tab enables automatically.
    </div>
  );
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
