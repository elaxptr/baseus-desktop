import { For } from 'solid-js';

/// A card with a label, a description and an on/off switch.
export function SwitchRow(p: {
  icon: string;
  label: string;
  desc: string;
  value: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <button
      class="lift press"
      onClick={() => p.onChange(!p.value)}
      style={{
        width: '100%',
        display: 'flex',
        'align-items': 'center',
        gap: '12px',
        background: p.value ? 'rgba(99,102,241,0.08)' : '#111113',
        border: `1px solid ${p.value ? 'rgba(99,102,241,0.4)' : '#1a1a1e'}`,
        'border-radius': '12px',
        padding: '12px 16px',
        cursor: 'pointer',
        'text-align': 'left',
        transition: 'border-color 0.12s, background 0.12s, transform 0.16s, box-shadow 0.16s',
      }}
    >
      <span style={{ 'font-size': '18px', width: '28px', 'text-align': 'center' }}>{p.icon}</span>
      <div style={{ flex: '1' }}>
        <div style={{ 'font-size': '13px', 'font-weight': '600', color: p.value ? '#c7d2fe' : '#aaa' }}>{p.label}</div>
        <div style={{ 'font-size': '10px', color: '#444', 'margin-top': '2px' }}>{p.desc}</div>
      </div>
      <div
        style={{
          width: '36px',
          height: '20px',
          'border-radius': '99px',
          background: p.value ? '#6366f1' : '#222',
          position: 'relative',
          transition: 'background 0.2s',
          'flex-shrink': '0',
        }}
      >
        <div
          style={{
            position: 'absolute',
            width: '16px',
            height: '16px',
            'border-radius': '50%',
            background: '#fff',
            top: '2px',
            left: p.value ? '18px' : '2px',
            transition: 'left 0.2s',
            'box-shadow': '0 1px 4px rgba(0,0,0,0.4)',
          }}
        />
      </div>
    </button>
  );
}

/// A row of mutually exclusive options, e.g. Off / 1 / 2 / 3.
export function Segmented<T extends string | number>(p: {
  options: { value: T; label: string }[];
  value: T;
  onChange: (v: T) => void;
}) {
  return (
    <div
      style={{
        display: 'flex',
        gap: '4px',
        background: '#111113',
        border: '1px solid #1a1a1e',
        'border-radius': '12px',
        padding: '4px',
      }}
    >
      <For each={p.options}>
        {(opt) => {
          const active = () => p.value === opt.value;
          return (
            <button
              class="press"
              onClick={() => p.onChange(opt.value)}
              style={{
                flex: '1',
                padding: '8px 0',
                'border-radius': '9px',
                border: 'none',
                background: active() ? 'rgba(99,102,241,0.18)' : 'transparent',
                color: active() ? '#c7d2fe' : '#666',
                'font-size': '12px',
                'font-weight': '600',
                cursor: 'pointer',
                transition: 'background 0.12s, color 0.12s',
              }}
            >
              {opt.label}
            </button>
          );
        }}
      </For>
    </div>
  );
}
