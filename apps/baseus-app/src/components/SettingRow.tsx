interface Props {
  label: string;
  description: string;
  value: boolean;
  onChange: (v: boolean) => void;
}

export default function SettingRow(props: Props) {
  return (
    <div
      style={{
        display: 'flex',
        'align-items': 'center',
        'justify-content': 'space-between',
        padding: '10px 0',
      }}
    >
      <div>
        <div style={{ 'font-size': '13px', color: '#a3a3a3' }}>{props.label}</div>
        <div style={{ 'font-size': '11px', color: '#525252', 'margin-top': '1px' }}>{props.description}</div>
      </div>

      <div
        style={{
          width: '36px',
          height: '20px',
          background: props.value ? '#6366f1' : '#1e1e22',
          'border-radius': '10px',
          position: 'relative',
          'flex-shrink': '0',
          cursor: 'pointer',
          transition: 'background 0.15s',
        }}
        onClick={() => props.onChange(!props.value)}
      >
        <div
          style={{
            position: 'absolute',
            width: '14px',
            height: '14px',
            background: '#fff',
            'border-radius': '50%',
            top: '3px',
            left: props.value ? '19px' : '3px',
            transition: 'left 0.15s',
          }}
        />
      </div>
    </div>
  );
}
