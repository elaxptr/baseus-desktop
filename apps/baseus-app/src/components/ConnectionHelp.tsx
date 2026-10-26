import { For, Show } from 'solid-js';
import { openUrl } from '@tauri-apps/plugin-opener';
import BluetoothOff from 'lucide-solid/icons/bluetooth-off';
import BluetoothSearching from 'lucide-solid/icons/bluetooth-searching';
import type { ConnectionIssue } from '../lib/tauri';

const REPO = 'https://github.com/elaxptr/baseus-desktop';
const ADD_MODEL_URL = `${REPO}/blob/main/docs/capture-your-model.md`;
const NEW_ISSUE_URL = `${REPO}/issues/new`;

interface Props {
  issue: ConnectionIssue | null;
}

/// Shown in place of the batteries while no earbuds are connected. The backend retries
/// forever, so without this a failing search looks identical to a slow one (issues #8, #10).
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
    <div class="card help">
      <div class="help-icon" classList={{ searching: !props.issue }}>
        <Show when={props.issue?.kind === 'adapter'} fallback={<BluetoothSearching size={20} />}>
          <BluetoothOff size={20} />
        </Show>
      </div>

      <div class="help-body">
        <div class="help-title">{title()}</div>

        <Show when={!props.issue}>
          <p>Keep the earbuds out of the case, close by and paired with this computer.</p>
        </Show>

        <Show when={props.issue?.kind === 'adapter'}>
          <p>Turn Bluetooth on. The app retries on its own every few seconds.</p>
        </Show>

        <Show when={props.issue?.unsupported.length}>
          <p>
            <For each={props.issue?.unsupported}>
              {(name, i) => (
                <>
                  {i() > 0 && ', '}
                  <b>{name}</b>
                </>
              )}
            </For>{' '}
            is nearby, but only {props.issue?.supported.join(' and ')} are supported so far. Support
            for each model is added from captures made by its owners.
          </p>
          <div class="help-actions">
            <button class="btn primary" onClick={() => openUrl(ADD_MODEL_URL).catch(() => {})}>
              Help add this model
            </button>
            <button class="btn" onClick={() => openUrl(NEW_ISSUE_URL).catch(() => {})}>
              Open an issue
            </button>
          </div>
        </Show>

        <Show when={props.issue?.kind === 'connect_failed'}>
          <p>
            The earbuds accept only one control connection at a time. Close the Baseus app on your
            phone (or turn off the phone's Bluetooth) and wait for the next retry. In dual-device
            mode, the other device may be holding that connection.
          </p>
        </Show>

        <Show when={props.issue?.kind === 'not_found' && !props.issue?.unsupported.length}>
          <ul>
            <li>Take the buds out of the case, or open the lid.</li>
            <li>Pair them with this computer in the system Bluetooth settings first.</li>
            <li>Close the Baseus app on your phone; it can hold the only control connection.</li>
          </ul>
        </Show>

        <Show when={props.issue}>
          <div class="help-detail">{props.issue?.detail}</div>
        </Show>
      </div>
    </div>
  );
}
