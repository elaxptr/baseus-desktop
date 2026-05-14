export type Tab = 'home' | 'anc' | 'eq' | 'gestures' | 'settings';

interface Props {
  active: Tab;
  onSwitch: (tab: Tab) => void;
}

const NAV: Array<{ tab: Tab; icon: string; label: string }> = [
  { tab: 'home',     icon: '⊙',  label: 'Battery' },
  { tab: 'anc',      icon: '◎',  label: 'Noise Control' },
  { tab: 'eq',       icon: '≋',  label: 'EQ' },
  { tab: 'gestures', icon: '⊡',  label: 'Gestures' },
];

export default function Sidebar(props: Props) {
  return (
    <div
      style={{
        width: '52px',
        background: '#0a0a0c',
        'border-right': '1px solid #161618',
        display: 'flex',
        'flex-direction': 'column',
        'align-items': 'center',
        padding: '10px 0',
        gap: '2px',
        'flex-shrink': '0',
      }}
    >
      {NAV.map(({ tab, icon, label }) => (
        <button
          title={label}
          onClick={() => props.onSwitch(tab)}
          style={{
            width: '36px',
            height: '36px',
            'border-radius': '9px',
            border: 'none',
            background: props.active === tab ? 'rgba(99,102,241,0.18)' : 'transparent',
            color: props.active === tab ? '#a5b4fc' : '#444',
            'font-size': '18px',
            cursor: 'pointer',
            display: 'flex',
            'align-items': 'center',
            'justify-content': 'center',
            position: 'relative',
            transition: 'background 0.12s, color 0.12s',
          }}
        >
          {props.active === tab && (
            <div
              style={{
                position: 'absolute',
                left: '-8px',
                width: '3px',
                height: '18px',
                background: '#6366f1',
                'border-radius': '0 3px 3px 0',
              }}
            />
          )}
          {icon}
        </button>
      ))}

      {/* Spacer + settings at bottom */}
      <div style={{ flex: '1' }} />
      <button
        title="Settings"
        onClick={() => props.onSwitch('settings')}
        style={{
          width: '36px',
          height: '36px',
          'border-radius': '9px',
          border: 'none',
          background: props.active === 'settings' ? 'rgba(99,102,241,0.18)' : 'transparent',
          color: props.active === 'settings' ? '#a5b4fc' : '#444',
          'font-size': '18px',
          cursor: 'pointer',
          display: 'flex',
          'align-items': 'center',
          'justify-content': 'center',
          position: 'relative',
          transition: 'background 0.12s, color 0.12s',
        }}
      >
        {props.active === 'settings' && (
          <div
            style={{
              position: 'absolute',
              left: '-8px',
              width: '3px',
              height: '18px',
              background: '#6366f1',
              'border-radius': '0 3px 3px 0',
            }}
          />
        )}
        ⚙
      </button>
    </div>
  );
}
