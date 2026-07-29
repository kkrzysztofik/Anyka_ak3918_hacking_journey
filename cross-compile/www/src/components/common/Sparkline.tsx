import { useId, useMemo } from 'react';

export interface SparklineSeries {
  key: string;
  label: string;
  color: string;
  unit?: string;
}

export interface SparklineProps {
  data: Array<Record<string, number>>;
  series: SparklineSeries[];
  /** Fixed value range. Omit to scale to the data. */
  domain?: [number, number];
  className?: string;
}

const VIEW_W = 300;
const VIEW_H = 100;

function buildPaths(
  data: Array<Record<string, number>>,
  seriesKey: string,
  min: number,
  max: number,
) {
  const span = max - min || 1;
  const step = data.length > 1 ? VIEW_W / (data.length - 1) : 0;
  const coords = data.map((point, i) => {
    const y = VIEW_H - ((point[seriesKey] - min) / span) * VIEW_H;
    return `${(i * step).toFixed(2)},${y.toFixed(2)}`;
  });
  const line = `M${coords.join('L')}`;
  return { line, area: `${line}L${VIEW_W},${VIEW_H}L0,${VIEW_H}Z` };
}

export function Sparkline({ data, series, domain, className }: Readonly<SparklineProps>) {
  const gradientId = useId();

  const bounds = useMemo<[number, number]>(() => {
    if (domain) return domain;
    const values = data.flatMap((point) => series.map((s) => point[s.key]));
    return values.length ? [Math.min(...values), Math.max(...values)] : [0, 1];
  }, [data, series, domain]);

  if (data.length === 0) return null;

  return (
    <svg
      viewBox={`0 0 ${VIEW_W} ${VIEW_H}`}
      preserveAspectRatio="none"
      className={className ?? 'h-full w-full'}
      role="img"
      aria-label={series.map((s) => s.label).join(', ')}
    >
      <defs>
        {series.map((s) => (
          <linearGradient key={s.key} id={`${gradientId}-${s.key}`} x1="0" y1="0" x2="0" y2="1">
            <stop offset="5%" stopColor={s.color} stopOpacity={0.3} />
            <stop offset="95%" stopColor={s.color} stopOpacity={0} />
          </linearGradient>
        ))}
      </defs>

      {[0.25, 0.5, 0.75].map((fraction) => (
        <line
          key={fraction}
          x1={0}
          x2={VIEW_W}
          y1={VIEW_H * fraction}
          y2={VIEW_H * fraction}
          stroke="#333"
          strokeDasharray="3 3"
          strokeWidth={0.5}
          vectorEffect="non-scaling-stroke"
        />
      ))}

      {series.map((s) => {
        const { line, area } = buildPaths(data, s.key, bounds[0], bounds[1]);
        return (
          <g key={s.key}>
            <path d={area} fill={`url(#${gradientId}-${s.key})`} data-testid="sparkline-area" />
            <path
              d={line}
              fill="none"
              stroke={s.color}
              strokeWidth={2}
              vectorEffect="non-scaling-stroke"
            />
          </g>
        );
      })}
    </svg>
  );
}
