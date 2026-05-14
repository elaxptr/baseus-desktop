interface Props {
  icon: string;
  label: string;
  active: boolean;
  loading: boolean;
  onClick: () => void;
}

export default function AncButton(props: Props) {
  const baseStyle = {
    flex: '1',
    padding: '10px 6px',
    'border-radius': '10px',
    'text-align': 'center' as const,
    'font-size': '12px',
    'font-weight': '500',
    cursor: 'pointer',
    display: 'flex',
    'flex-direction': 'column' as const,
    'align-items': 'center',
    gap: '4px',
    border: '1px solid',
    transition: 'all 0.12s',
  };

  const activeStyle = {
    background: 'rgba(99,102,241,0.12)',
    'border-color': 'rgba(99,102,241,0.3)',
    color: '#a5b4fc',
  };

  const inactiveStyle = {
    background: '#111113',
    'border-color': '#1a1a1e',
    color: '#404040',
  };

  return (
    <div
      style={{
        ...baseStyle,
        ...(props.active ? activeStyle : inactiveStyle),
        animation: props.loading ? 'pulse 0.8s ease-in-out infinite' : 'none',
      }}
      onClick={props.onClick}
    >
      <span style={{ 'font-size': '16px' }}>{props.icon}</span>
      <span style={{ color: props.active ? '#818cf8' : '#404040' }}>{props.label}</span>
    </div>
  );
}
