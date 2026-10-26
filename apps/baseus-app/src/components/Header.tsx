import { Show } from 'solid-js';
import ChevronLeft from 'lucide-solid/icons/chevron-left';
import Settings from 'lucide-solid/icons/settings';
import logo from '../assets/logo.svg';

interface Props {
  title: string;
  status: string;
  connected: boolean;
  settingsOpen: boolean;
  updateAvailable: boolean;
  onSettings: () => void;
  onBack: () => void;
}

export default function Header(props: Props) {
  return (
    <header class="header">
      <Show
        when={props.settingsOpen}
        fallback={<img class="header-logo" src={logo} alt="" />}
      >
        <button class="icon-btn" onClick={props.onBack} aria-label="Back">
          <ChevronLeft size={20} />
        </button>
      </Show>

      <div class="header-text">
        <div class="header-title">{props.settingsOpen ? 'Settings' : props.title}</div>
        <Show when={!props.settingsOpen}>
          <div class="status" classList={{ connected: props.connected }}>
            <span class="status-dot" />
            {props.status}
          </div>
        </Show>
      </div>

      <Show when={!props.settingsOpen}>
        <button class="icon-btn" onClick={props.onSettings} aria-label="Settings" title="Settings">
          <Settings size={18} />
          <Show when={props.updateAvailable}>
            <span class="badge-dot" />
          </Show>
        </button>
      </Show>
    </header>
  );
}
