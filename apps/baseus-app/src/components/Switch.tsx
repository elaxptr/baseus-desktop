interface Props {
  on: boolean;
  label: string;
  onChange: (on: boolean) => void;
}

export default function Switch(props: Props) {
  return (
    <button
      class="switch"
      classList={{ on: props.on }}
      role="switch"
      aria-checked={props.on}
      aria-label={props.label}
      onClick={() => props.onChange(!props.on)}
    />
  );
}
