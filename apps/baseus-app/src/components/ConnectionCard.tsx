import { Component } from 'solid-js';

type Status = 'connected' | 'connecting' | 'disconnected';

interface Props {
  status: Status;
  lastUpdated: string | null;
}

const STATUS_COLOR: Record<Status, string> = {
  connected: 'bg-green-500',
  connecting: 'bg-yellow-500 animate-pulse',
  disconnected: 'bg-neutral-600',
};

const STATUS_LABEL: Record<Status, string> = {
  connected: 'Connected',
  connecting: 'Connecting…',
  disconnected: 'Disconnected',
};

const ConnectionCard: Component<Props> = (props) => (
  <div class="flex items-center gap-3 rounded-2xl bg-neutral-900 px-5 py-3 w-full max-w-sm">
    <span class={`w-3 h-3 rounded-full flex-shrink-0 ${STATUS_COLOR[props.status]}`} />
    <div class="flex flex-col min-w-0">
      <span class="text-sm font-semibold">{STATUS_LABEL[props.status]}</span>
      {props.lastUpdated && (
        <span class="text-xs text-neutral-500 truncate">Updated {props.lastUpdated}</span>
      )}
    </div>
  </div>
);

export default ConnectionCard;
