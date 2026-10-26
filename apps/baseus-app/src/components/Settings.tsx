import { createSignal, onMount, Show } from 'solid-js';
import { getVersion } from '@tauri-apps/api/app';
import { openUrl } from '@tauri-apps/plugin-opener';
import Bell from 'lucide-solid/icons/bell';
import Clock from 'lucide-solid/icons/clock';
import ExternalLink from 'lucide-solid/icons/external-link';
import Power from 'lucide-solid/icons/power';
import Switch from './Switch';
import logo from '../assets/logo.svg';
import { getSettingsStore, updateSetting } from '../stores/settings';
import { checkForUpdate, installUpdate, type FirmwareVersion, type Settings as SettingsData } from '../lib/tauri';

const REPO = 'https://github.com/elaxptr/baseus-desktop';

interface Props {
  initialUpdateVersion: string | null;
  onUpdateInstalled: () => void;
  /** The connected earbuds' firmware, when the model reports it. */
  firmware: FirmwareVersion | null;
}

type UpdateState = 'idle' | 'checking' | 'up-to-date' | 'available' | 'installing' | 'error';

const PREFERENCES: { key: keyof SettingsData; title: string; desc: string; icon: typeof Bell }[] = [
  { key: 'launch_at_login', title: 'Launch at login', desc: 'Start in the tray when you sign in', icon: Power },
  { key: 'low_battery_alerts', title: 'Low battery alerts', desc: 'Notify when a bud or the case drops below 20%', icon: Bell },
  { key: 'show_session_timer', title: 'Session timer', desc: 'Show how long the earbuds have been connected', icon: Clock },
];

export default function Settings(props: Props) {
  const [appVersion, setAppVersion] = createSignal('');
  const [updateState, setUpdateState] = createSignal<UpdateState>('idle');
  const [availableVersion, setAvailableVersion] = createSignal<string | null>(props.initialUpdateVersion);
  // Last check/install failure. Shown until the next attempt so a failure isn't silent.
  const [updateError, setUpdateError] = createSignal<string | null>(null);

  onMount(async () => {
    setAppVersion(await getVersion());
    // If the background check already found an update, reflect that immediately.
    if (props.initialUpdateVersion) setUpdateState('available');
  });

  async function handleCheck() {
    setUpdateState('checking');
    setUpdateError(null);
    try {
      const version = await checkForUpdate();
      if (version) {
        setAvailableVersion(version);
        setUpdateState('available');
      } else {
        setUpdateState('up-to-date');
      }
    } catch (e) {
      setUpdateError(`Couldn't check for updates: ${e}`);
      setUpdateState('error');
    }
  }

  async function handleInstall() {
    setUpdateState('installing');
    setUpdateError(null);
    try {
      await installUpdate();
      props.onUpdateInstalled();
    } catch (e) {
      setUpdateError(`Couldn't install the update: ${e}`);
      setUpdateState('available');
    }
  }

  const firmwareText = () => {
    const fw = props.firmware;
    if (!fw) return null;
    return fw.left === fw.right ? fw.left : `L ${fw.left} · R ${fw.right}`;
  };

  // The install panel stays up while installing; the check button only replaces it after.
  const showInstall = () => updateState() === 'available' || updateState() === 'installing';

  return (
    <>
      <section>
        <h2 class="section-title">Preferences</h2>
        <div class="card rows">
          {PREFERENCES.map(({ key, title, desc, icon: Icon }) => (
            <div class="row">
              <div class="row-icon">
                <Icon size={18} />
              </div>
              <div class="row-text">
                <div class="row-title">{title}</div>
                <div class="row-desc">{desc}</div>
              </div>
              <Switch on={getSettingsStore()[key]} label={title} onChange={(v) => updateSetting(key, v)} />
            </div>
          ))}
        </div>
      </section>

      <section>
        <h2 class="section-title">About</h2>
        <div class="card">
          <div class="about">
            <img src={logo} alt="" />
            <div class="row-text">
              <div class="row-title">Baseus Desktop</div>
              <div class="row-desc num">{appVersion() ? `Version ${appVersion()}` : '…'}</div>
              <Show when={firmwareText()}>
                <div class="row-desc num">Earbuds firmware {firmwareText()}</div>
              </Show>
            </div>
            <Show when={!showInstall()}>
              <button class="btn" onClick={handleCheck} disabled={updateState() === 'checking'}>
                {updateState() === 'checking' ? 'Checking…' :
                 updateState() === 'up-to-date' ? 'Up to date' :
                 updateState() === 'error' ? 'Retry' : 'Check for updates'}
              </button>
            </Show>
          </div>

          <Show when={showInstall()}>
            <div class="update">
              <div class="row-text">
                <div class="row-title">Update available</div>
                <div class="row-desc num">Version {availableVersion()}</div>
              </div>
              <button class="btn primary" onClick={handleInstall} disabled={updateState() === 'installing'}>
                {updateState() === 'installing' ? 'Installing…' : 'Install and restart'}
              </button>
            </div>
          </Show>

          <Show when={updateError()}>
            <div class="error-text">{updateError()}</div>
          </Show>

          <div class="rows" style={{ 'border-top': '1px solid var(--border)' }}>
            <button class="row" onClick={() => openUrl(REPO).catch(() => {})}>
              <div class="row-text row-title">Source code and issues</div>
              <ExternalLink size={16} />
            </button>
          </div>
        </div>
      </section>
    </>
  );
}
