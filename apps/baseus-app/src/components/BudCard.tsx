import SparkLine from './SparkLine';

interface Props {
  label: string;
  pct: number;
  history: number[];
}

const CIRCUMFERENCE = 2 * Math.PI * 32; // r=32

export default function BudCard(props: Props) {
  const isLow = () => props.pct > 0 && props.pct < 20;
  const color = () => (isLow() ? '#ef4444' : '#22c55e');
  const offset = () => CIRCUMFERENCE * (1 - props.pct / 100);

  return (
    <div
      style={{
        flex: '1',
        background: '#111113',
        border: '1px solid #1a1a1e',
        'border-radius': '14px',
        padding: '16px 12px 12px',
        display: 'flex',
        'flex-direction': 'column',
        'align-items': 'center',
        gap: '10px',
      }}
    >
      <div style={{ position: 'relative', width: '80px', height: '80px' }}>
        <svg
          width="80"
          height="80"
          viewBox="0 0 80 80"
          style={{ transform: 'rotate(-90deg)' }}
        >
          <circle cx="40" cy="40" r="32" fill="none" stroke="#1a1a1e" stroke-width="6" />
          <circle
            cx="40"
            cy="40"
            r="32"
            fill="none"
            stroke={color()}
            stroke-width="6"
            stroke-dasharray={String(CIRCUMFERENCE)}
            stroke-dashoffset={offset()}
            stroke-linecap="round"
            style={{ transition: 'stroke-dashoffset 0.5s ease, stroke 0.3s ease' }}
          />
        </svg>
        <div
          style={{
            position: 'absolute',
            inset: '0',
            display: 'flex',
            'flex-direction': 'column',
            'align-items': 'center',
            'justify-content': 'center',
          }}
        >
          <span style={{ 'font-size': '22px', 'font-weight': '800', color: '#fff', 'line-height': '1' }}>
            {props.pct}
          </span>
          <span style={{ 'font-size': '10px', color: '#404040', 'font-weight': '500' }}>%</span>
        </div>
      </div>

      <SparkLine data={props.history} color={color()} width={88} height={24} />

      <span style={{ 'font-size': '11px', color: '#525252', 'font-weight': '500', 'letter-spacing': '0.04em' }}>
        {props.label}
      </span>
    </div>
  );
}
