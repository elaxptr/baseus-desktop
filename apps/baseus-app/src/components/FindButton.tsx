import { createSignal } from 'solid-js';

interface Props {
  label: string;
  onClick: () => void;
}

export default function FindButton(props: Props) {
  const [loading, setLoading] = createSignal(false);

  function handleClick() {
    if (loading()) return;
    setLoading(true);
    props.onClick();
    setTimeout(() => setLoading(false), 3000);
  }

  return (
    <div
      style={{
        flex: '1',
        display: 'flex',
        'align-items': 'center',
        'justify-content': 'center',
        gap: '7px',
        padding: '11px',
        background: '#111113',
        border: '1px solid #1a1a1e',
        'border-radius': '10px',
        'font-size': '12px',
        'font-weight': '500',
        color: loading() ? '#a3a3a3' : '#525252',
        cursor: loading() ? 'default' : 'pointer',
        transition: 'color 0.12s',
      }}
      onClick={handleClick}
    >
      <span style={{ 'font-size': '14px' }}>🔊</span>
      {loading() ? 'Playing…' : props.label}
    </div>
  );
}
