const TAPS = ['Double Tap', 'Triple Tap', 'Long Press'] as const;

const DEFAULTS: Record<typeof TAPS[number], [string, string]> = {
  'Double Tap': ['Play / Pause', 'Play / Pause'],
  'Triple Tap': ['Next Track',   'Prev Track'],
  'Long Press': ['ANC Toggle',   'Voice Assistant'],
};

export default function GesturesTab() {
  return (
    <div>
      {(['Left', 'Right'] as const).map((side, si) => (
        <div style={{ 'margin-bottom': '10px' }}>
          <div style={labelStyle}>{side} Bud <Divider /></div>
          <div
            style={{
              background: '#111113',
              border: '1px solid #1a1a1e',
              'border-radius': '12px',
              padding: '4px 14px',
              opacity: '0.5',
            }}
          >
            {TAPS.map((tap) => (
              <div
                style={{
                  display: 'flex',
                  'align-items': 'center',
                  'justify-content': 'space-between',
                  padding: '10px 0',
                  'border-bottom': tap !== 'Long Press' ? '1px solid #161618' : 'none',
                }}
              >
                <span style={{ 'font-size': '10px', color: '#444', 'font-weight': '600', 'letter-spacing': '0.04em', 'text-transform': 'uppercase' }}>
                  {tap}
                </span>
                <div
                  style={{
                    'font-size': '11px',
                    color: '#777',
                    background: '#161618',
                    border: '1px solid #222',
                    'border-radius': '6px',
                    padding: '4px 10px',
                    cursor: 'not-allowed',
                  }}
                >
                  {DEFAULTS[tap][si]} ▾
                </div>
              </div>
            ))}
          </div>
        </div>
      ))}
      <ReNotice />
    </div>
  );
}

function ReNotice() {
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
      Gesture remapping requires capturing the BLE write bytes from the Android app via Frida.
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
