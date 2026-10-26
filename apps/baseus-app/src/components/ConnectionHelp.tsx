import { For, Show } from 'solid-js';
import { openUrl } from '@tauri-apps/plugin-opener';
import type { ConnectionIssue } from '../lib/tauri';

const REPO = 'https://github.com/elaxptr/baseus-desktop';
const ADD_MODEL_URL = `${REPO}/blob/main/docs/re-methodology.md`;
const NEW_ISSUE_URL = `${REPO}/issues/new`;

interface Props {
  connecting: boolean;
  issue: ConnectionIssue | null;
}

/// Shown on the home tab while no earbuds are connected. The backend retries forever,
/// so without this a failing search looks identical to a slow one (issues #8, #10).
export default function ConnectionHelp(props: Props) {
  const title = () => {
    const issue = props.issue;
    if (issue?.kind === 'adapter') return 'Bluetooth is unavailable';
    if (issue?.unsupported.length) return 'Unsupported earbuds found';
    if (issue?.kind === 'connect_failed') return "Couldn't open the earbuds' control channel";
    if (issue?.kind === 'not_found') return 'No supported earbuds found';
    return 'Looking for your earbuds…';
  };

  return (
    <div
      style={{
        background: '#111113',
        border: '1px solid #1a1a1e',
        'border-radius': '14px',
        padding: '14px',
        'margin-bottom': '14px',
        'font-size': '12px',
        color: '#999',
        'line-height': '1.5',
      }}
    >
      <div style={{ display: 'flex', 'align-items': 'center', gap: '8px', 'margin-bottom': '6px' }}>
        <div
          class={props.connecting ? 'status-dot' : undefined}
          style={{
            width: '6px',
            height: '6px',
            'border-radius': '50%',
            background: props.connecting ? '#eab308' : '#525252',
            'flex-shrink': '0',
          }}
        />
        <span style={{ 'font-weight': '600', color: '#ddd' }}>{title()}</span>
      </div>

      <Show when={props.issue?.kind === 'adapter'}>
        <p style={pStyle}>Turn Bluetooth on. The app retries on its own every few seconds.</p>
      </Show>

      <Show when={props.issue?.unsupported.length}>
        <p style={pStyle}>
          <For each={props.issue?.unsupported}>
            {(name, i) => (
              <>
                {i() > 0 && ', '}
                <b style={{ color: '#ddd' }}>{name}</b>
              </>
            )}
          </For>{' '}
          is nearby, but only {props.issue?.supported.join(' and ')} are supported so far. Support
          for each model is added from captures made by its owners.
        </p>
        <div style={{ display: 'flex', gap: '8px', 'margin-top': '8px' }}>
          <LinkBtn label="Help add this model" url={ADD_MODEL_URL} />
          <LinkBtn label="Open an issue" url={NEW_ISSUE_URL} />
        </div>
      </Show>

      <Show when={props.issue?.kind === 'connect_failed'}>
        <p style={pStyle}>
          The earbuds were found, but they accept only one control connection at a time. Close the
          Baseus app on your phone (or turn off the phone's Bluetooth) and wait for the next retry.
          In dual-device mode, the other device may be holding that connection.
        </p>
      </Show>

      <Show when={props.issue?.kind === 'not_found' && !props.issue?.unsupported.length}>
        <ul style={ulStyle}>
          <li>Take the buds out of the case, or open the lid.</li>
          <li>Pair them with this computer in the system Bluetooth settings first.</li>
          <li>Close the Baseus app on your phone; it can hold the only control connection.</li>
        </ul>
      </Show>

      <Show when={props.issue}>
        <div class="num-mono" style={{ 'margin-top': '8px', 'font-size': '10px', color: '#444', 'word-break': 'break-word' }}>
          {props.issue?.detail}
        </div>
      </Show>
    </div>
  );
}

function LinkBtn(p: { label: string; url: string }) {
  return (
    <button
      class="lift"
      onClick={() => openUrl(p.url).catch(() => {})}
      style={{
        flex: '1',
        background: '#16161a',
        border: '1px solid #232328',
        'border-radius': '9px',
        padding: '7px 8px',
        color: '#818cf8',
        'font-size': '11px',
        'font-weight': '600',
        cursor: 'pointer',
      }}
    >
      {p.label}
    </button>
  );
}

const pStyle = { margin: '0' };
const ulStyle = { margin: '0', 'padding-left': '18px' };
