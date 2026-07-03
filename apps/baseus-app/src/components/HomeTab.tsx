import { createSignal, createEffect, onCleanup, untrack } from 'solid-js';
import SparkLine from './SparkLine';
import { findEarbud } from '../lib/tauri';
import { formatElapsed } from '../lib/timer';

interface Props {
  leftPct: number;
  rightPct: number;
  casePct: number;
  leftCharging: boolean;
  rightCharging: boolean;
  caseCharging: boolean;
  leftInEar: boolean;
  rightInEar: boolean;
  wearKnown: boolean; // false until a wear_update event arrives; greys out dots
  leftHistory: number[];
  rightHistory: number[];
  elapsed: number; // seconds
  showTimer: boolean;
}

const CIRC = 2 * Math.PI * 28;

function reduceMotion() {
  return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

/// Smoothly tween a displayed value toward `target`, re-running whenever it
/// changes (so rings sweep + count up on mount and on each new battery reading).
function useCountUp(target: () => number): () => number {
  const [shown, setShown] = createSignal(0);
  let raf = 0;
  createEffect(() => {
    const to = target();
    if (reduceMotion()) {
      setShown(to);
      return;
    }
    cancelAnimationFrame(raf);
    const from = untrack(shown);
    let startTs = 0;
    const dur = 900;
    const tick = (now: number) => {
      if (!startTs) startTs = now;
      const t = Math.min((now - startTs) / dur, 1);
      const eased = 1 - Math.pow(1 - t, 3);
      setShown(Math.round(from + (to - from) * eased));
      if (t < 1) raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
  });
  onCleanup(() => cancelAnimationFrame(raf));
  return shown;
}

function BudRing(p: {
  pct: number;
  charging: boolean;
  inEar: boolean;
  wearKnown: boolean;
  label: string;
  history: number[];
}) {
  const shown = useCountUp(() => p.pct);
  const isLow = () => p.pct > 0 && p.pct < 20;
  const color = () => (isLow() ? '#ef4444' : '#22c55e');
  const offset = () => CIRC * (1 - shown() / 100);
  const wearColor = () =>
    !p.wearKnown ? '#2a2a2a' : p.inEar ? '#22c55e' : '#3f3f3f';

  return (
    <div
      class="lift"
      style={{
        flex: '1',
        background: '#111113',
        border: '1px solid #1a1a1e',
        'border-radius': '14px',
        padding: '14px 10px 10px',
        display: 'flex',
        'flex-direction': 'column',
        'align-items': 'center',
        gap: '8px',
        position: 'relative',
      }}
    >
      {/* Wear detection dot */}
      <div
        title={!p.wearKnown ? 'Wear detection not yet available' : p.inEar ? 'In ear' : 'Not in ear'}
        style={{
          position: 'absolute',
          top: '10px',
          right: '10px',
          width: '7px',
          height: '7px',
          'border-radius': '50%',
          background: wearColor(),
          'box-shadow': p.wearKnown && p.inEar ? `0 0 6px ${wearColor()}` : 'none',
        }}
      />

      {/* Battery ring */}
      <div style={{ position: 'relative', width: '72px', height: '72px' }}>
        <svg width="72" height="72" viewBox="0 0 72 72" style={{ transform: 'rotate(-90deg)' }}>
          <circle cx="36" cy="36" r="28" fill="none" stroke="#1a1a1e" stroke-width="5" />
          <circle
            cx="36" cy="36" r="28" fill="none"
            stroke={color()} stroke-width="5"
            stroke-dasharray={String(CIRC)}
            stroke-dashoffset={String(offset())}
            stroke-linecap="round"
            style={{ transition: 'stroke 0.3s ease', filter: `drop-shadow(0 0 4px ${color()}99)` }}
          />
        </svg>
        <div
          style={{
            position: 'absolute', inset: '0',
            display: 'flex', 'flex-direction': 'column',
            'align-items': 'center', 'justify-content': 'center',
          }}
        >
          <span class="num-mono" style={{ 'font-size': '20px', 'font-weight': '800', color: '#fff', 'line-height': '1' }}>
            {shown()}
          </span>
          <span style={{ 'font-size': '9px', color: '#404040', 'font-weight': '500' }}>%</span>
        </div>
      </div>

      <SparkLine data={p.history} color={color()} width={80} height={22} />

      <div style={{ display: 'flex', 'align-items': 'center', gap: '4px' }}>
        <span style={{ 'font-size': '10px', color: '#444', 'font-weight': '600', 'letter-spacing': '0.05em' }}>
          {p.label}
        </span>
        {p.charging && (
          <span class="bolt-flash" style={{ 'font-size': '10px', color: '#eab308' }}>⚡</span>
        )}
      </div>
    </div>
  );
}

export default function HomeTab(props: Props) {
  const caseShown = useCountUp(() => props.casePct);
  const caseColor = () =>
    props.casePct > 0 && props.casePct < 20 ? '#ef4444' : '#eab308';

  return (
    <div>
      {/* Battery */}
      <div style={{ 'margin-bottom': '14px' }}>
        <div style={labelStyle}>Earbuds <Divider /></div>
        <div style={{ display: 'flex', gap: '10px' }}>
          <BudRing
            pct={props.leftPct} charging={props.leftCharging}
            inEar={props.leftInEar} wearKnown={props.wearKnown}
            label="LEFT" history={props.leftHistory}
          />
          <BudRing
            pct={props.rightPct} charging={props.rightCharging}
            inEar={props.rightInEar} wearKnown={props.wearKnown}
            label="RIGHT" history={props.rightHistory}
          />
        </div>
      </div>

      {/* Case */}
      <div style={{ 'margin-bottom': '14px' }}>
        <div style={labelStyle}>Case <Divider /></div>
        <div
          class="lift"
          style={{
            background: '#111113',
            border: '1px solid #1a1a1e',
            'border-radius': '12px',
            padding: '12px 14px',
            display: 'flex',
            'align-items': 'center',
            gap: '12px',
          }}
        >
          <span style={{ 'font-size': '20px', opacity: '0.5' }}>🗃️</span>
          <div style={{ flex: '1' }}>
            <div style={{ display: 'flex', 'justify-content': 'space-between', 'margin-bottom': '6px' }}>
              <span style={{ 'font-size': '11px', color: '#555' }}>Case</span>
              <span class="num-mono" style={{ 'font-size': '13px', 'font-weight': '800', color: caseColor() }}>
                {caseShown()}%
              </span>
            </div>
            <div style={{ height: '4px', background: '#1a1a1e', 'border-radius': '99px', overflow: 'hidden' }}>
              <div
                style={{
                  height: '100%',
                  width: `${caseShown()}%`,
                  background: `linear-gradient(90deg, ${caseColor()}, ${caseColor()}aa)`,
                  'border-radius': '99px',
                }}
              />
            </div>
          </div>
          {props.caseCharging && (
            <span class="bolt-flash" style={{ 'font-size': '16px', color: '#eab308' }}>⚡</span>
          )}
        </div>
      </div>

      {/* Find Earbuds */}
      <div style={{ 'margin-bottom': '14px' }}>
        <div style={labelStyle}>Find Earbuds <Divider /></div>
        <div style={{ display: 'flex', gap: '8px' }}>
          <FindBtn label="Play Left"  onClick={() => findEarbud('left').catch(() => {})} />
          <FindBtn label="Play Right" onClick={() => findEarbud('right').catch(() => {})} />
        </div>
      </div>

      {/* Session timer */}
      {props.showTimer && props.elapsed > 0 && (
        <div style={{ 'margin-bottom': '4px' }}>
          <div style={labelStyle}>Today <Divider /></div>
          <div
            class="lift"
            style={{
              background: '#111113',
              border: '1px solid #1a1a1e',
              'border-radius': '12px',
              padding: '12px 14px',
              display: 'flex',
              'align-items': 'center',
              gap: '10px',
            }}
          >
            <span style={{ 'font-size': '16px', opacity: '0.4' }}>⏱</span>
            <div>
              <div class="num-mono" style={{ 'font-size': '17px', 'font-weight': '800', color: '#fff' }}>
                {formatElapsed(props.elapsed)}
              </div>
              <div style={{ 'font-size': '10px', color: '#444', 'margin-top': '1px' }}>Current session</div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function Divider() {
  return <div style={{ flex: '1', height: '1px', background: '#161618' }} />;
}

function FindBtn(p: { label: string; onClick: () => void }) {
  function handleClick(e: MouseEvent) {
    const btn = e.currentTarget as HTMLElement;
    if (!reduceMotion()) {
      const rect = btn.getBoundingClientRect();
      const r = document.createElement('span');
      r.className = 'ripple';
      r.style.left = `${e.clientX - rect.left}px`;
      r.style.top = `${e.clientY - rect.top}px`;
      btn.appendChild(r);
      setTimeout(() => r.remove(), 600);
    }
    p.onClick();
  }

  return (
    <button
      class="lift press ripple-host"
      onClick={handleClick}
      style={{
        flex: '1',
        background: '#111113',
        border: '1px solid #1a1a1e',
        'border-radius': '10px',
        padding: '10px',
        color: '#888',
        'font-size': '12px',
        'font-weight': '500',
        cursor: 'pointer',
        transition: 'background 0.12s, border-color 0.12s, transform 0.16s, box-shadow 0.16s',
      }}
    >
      {p.label}
    </button>
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
