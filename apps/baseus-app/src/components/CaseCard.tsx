import SparkLine from './SparkLine';

interface Props {
  pct: number;
  history: number[];
}

const CIRCUMFERENCE = 2 * Math.PI * 20; // r=20

export default function CaseCard(props: Props) {
  const offset = () => CIRCUMFERENCE * (1 - props.pct / 100);

  return (
    <div
      style={{
        background: '#111113',
        border: '1px solid #1a1a1e',
        'border-radius': '14px',
        padding: '14px 16px',
        display: 'flex',
        'align-items': 'center',
        gap: '14px',
      }}
    >
      <div style={{ position: 'relative', width: '52px', height: '52px', 'flex-shrink': '0' }}>
        <svg
          width="52"
          height="52"
          viewBox="0 0 52 52"
          style={{ transform: 'rotate(-90deg)' }}
        >
          <circle cx="26" cy="26" r="20" fill="none" stroke="#1a1a1e" stroke-width="5" />
          <circle
            cx="26"
            cy="26"
            r="20"
            fill="none"
            stroke="#6366f1"
            stroke-width="5"
            stroke-dasharray={String(CIRCUMFERENCE)}
            stroke-dashoffset={offset()}
            stroke-linecap="round"
            style={{ transition: 'stroke-dashoffset 0.5s ease' }}
          />
        </svg>
        <div
          style={{
            position: 'absolute',
            inset: '0',
            display: 'flex',
            'align-items': 'center',
            'justify-content': 'center',
            'font-size': '14px',
            'font-weight': '700',
            color: '#fff',
          }}
        >
          {props.pct}
        </div>
      </div>

      <div style={{ flex: '1' }}>
        <div style={{ 'font-size': '11px', color: '#525252', 'font-weight': '500', 'letter-spacing': '0.04em', 'margin-bottom': '6px' }}>
          CASE BATTERY
        </div>
        <SparkLine data={props.history} color="#6366f1" width={160} height={20} />
      </div>
    </div>
  );
}
