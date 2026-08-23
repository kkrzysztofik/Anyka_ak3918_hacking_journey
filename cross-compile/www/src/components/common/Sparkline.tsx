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

/** "CPU 42%, Memory 61%" — the label alone conveys nothing to a screen reader. */
function describe(data: Array<Record<string, number>>, series: SparklineSeries[]) {
  const latest = data.at(-1);
  return series
    .map((s) => {
      const value = latest?.[s.key];
      return value === undefined ? s.label : `${s.label} ${value}${s.unit ?? ''}`;
    })
    .join(', ');
}

function formatLegendValue(value: number | undefined, unit?: string): string | null {
  if (value === undefined) return null;
  const rounded = Number.isInteger(value) ? String(value) : value.toFixed(1);
  return `${rounded}${unit ?? ''}`;
}

export function Sparkline({ data, series, domain, className }: Readonly<SparklineProps>) {
  const gradientId = useId();

  const bounds = useMemo<[number, number]>(() => {
    if (domain) return domain;
    const values = data.flatMap((point) => series.map((s) => point[s.key]));
    return values.length ? [Math.min(...values), Math.max(...values)] : [0, 1];
  }, [data, series, domain]);

  if (data.length === 0) return null;

  const latest = data.at(-1);

  return (
    <div className={className ?? 'flex h-full w-full flex-col'} data-testid="sparkline">
      <svg
        viewBox={`0 0 ${VIEW_W} ${VIEW_H}`}
        preserveAspectRatio="none"
        className="min-h-0 w-full flex-1"
        role="img"
        aria-label={describe(data, series)}
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

      <ul
        className="text-muted-foreground mt-2 flex flex-wrap items-center gap-x-4 gap-y-1 text-xs"
        data-testid="sparkline-legend"
      >
        {series.map((s) => {
          const valueText = formatLegendValue(latest?.[s.key], s.unit);
          return (
            <li
              key={s.key}
              className="flex items-center gap-1.5"
              data-testid={`sparkline-legend-${s.key}`}
            >
              <span
                className="h-2 w-2 shrink-0 rounded-full"
                style={{ backgroundColor: s.color }}
                aria-hidden
              />
              <span>{s.label}</span>
              {valueText !== null && (
                <span
                  className="text-foreground font-mono"
                  data-testid={`sparkline-legend-${s.key}-value`}
                >
                  {valueText}
                </span>
              )}
            </li>
          );
        })}
      </ul>
    </div>
  );
}
