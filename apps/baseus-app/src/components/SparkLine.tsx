interface Props {
  data: number[];
  color: string;
  width?: number;
  height?: number;
}

export default function SparkLine(props: Props) {
  const w = () => props.width ?? 100;
  const h = () => props.height ?? 24;

  const points = () => {
    const d = props.data;
    if (d.length < 2) {
      // Flat baseline at 50% height as a placeholder
      return `0,${(h() / 2).toFixed(1)} ${w().toFixed(1)},${(h() / 2).toFixed(1)}`;
    }
    const step = w() / (d.length - 1);
    return d
      .map((v, i) => {
        const x = i * step;
        const y = h() - (v / 100) * h();
        return `${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(' ');
  };

  const opacity = () => (props.data.length < 2 ? 0.15 : 0.5);

  return (
    <svg
      width={w()}
      height={h()}
      viewBox={`0 0 ${w()} ${h()}`}
      style="overflow: visible;"
    >
      <polyline
        points={points()}
        fill="none"
        stroke={props.color}
        stroke-width="1.5"
        stroke-linecap="round"
        stroke-linejoin="round"
        opacity={opacity()}
      />
    </svg>
  );
}
