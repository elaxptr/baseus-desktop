import { getSettingsStore, updateSetting } from '../stores/settings';

export default function SettingsTab() {
  return (
    <div>
      <div style={labelStyle}>Preferences <Divider /></div>
      <Toggle
        label="Launch at login"
        desc="Start automatically with Windows"
        value={getSettingsStore().launch_at_login}
        onChange={(v) => updateSetting('launch_at_login', v)}
      />
      <div style={{ height: '1px', background: '#131315' }} />
      <Toggle
        label="Low battery alerts"
        desc="Notify when a bud drops below 20%"
        value={getSettingsStore().low_battery_alerts}
        onChange={(v) => updateSetting('low_battery_alerts', v)}
      />
      <div style={{ height: '1px', background: '#131315' }} />
      <Toggle
        label="Show session timer"
        desc="Display listening time on the home tab"
        value={getSettingsStore().show_session_timer}
        onChange={(v) => updateSetting('show_session_timer', v)}
      />
    </div>
  );
}

function Toggle(p: { label: string; desc: string; value: boolean; onChange: (v: boolean) => void }) {
  return (
    <div style={{ display: 'flex', 'align-items': 'center', padding: '14px 0' }}>
      <div style={{ flex: '1' }}>
        <div style={{ 'font-size': '13px', color: '#ccc', 'font-weight': '500' }}>{p.label}</div>
        <div style={{ 'font-size': '10px', color: '#444', 'margin-top': '2px' }}>{p.desc}</div>
      </div>
      <div
        onClick={() => p.onChange(!p.value)}
        style={{
          width: '36px', height: '20px',
          'border-radius': '99px',
          background: p.value ? '#22c55e' : '#222',
          position: 'relative',
          cursor: 'pointer',
          transition: 'background 0.2s',
          'flex-shrink': '0',
        }}
      >
        <div
          style={{
            position: 'absolute',
            width: '16px', height: '16px',
            'border-radius': '50%',
            background: '#fff',
            top: '2px',
            left: p.value ? 'auto' : '2px',
            right: p.value ? '2px' : 'auto',
            transition: 'left 0.2s, right 0.2s',
            'box-shadow': '0 1px 4px rgba(0,0,0,0.4)',
          }}
        />
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
