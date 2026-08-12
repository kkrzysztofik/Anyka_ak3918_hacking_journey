import { useState } from 'react';

import { useQuery } from '@tanstack/react-query';

import { type Diagnostics, getDiagnostics } from '@/services/diagnosticsService';

/** 60 samples × 5 s poll = 5 minutes of history. */
const MAX_SAMPLES = 60;
const POLL_MS = 5000;

export interface DiagnosticsPoint {
  /** Timestamp from TanStack Query's dataUpdatedAt (ms since epoch). */
  t: number;
  /** CPU usage percentage, or null when the metric is unavailable. */
  cpu: number | null;
  /** Memory usage as a percentage of total, or null when unavailable. */
  memPct: number | null;
  /** Network receive rate in bytes/sec, or null when unavailable. */
  rx: number | null;
  /** Network transmit rate in bytes/sec, or null when unavailable. */
  tx: number | null;
}

function toPoint(data: Diagnostics, t: number): DiagnosticsPoint {
  const memPct =
    data.memory !== null && data.memory.total_kb > 0
      ? (data.memory.used_kb / data.memory.total_kb) * 100
      : null;

  return {
    t,
    cpu: data.cpu_percent,
    memPct,
    rx: data.network?.rx_bps ?? null,
    tx: data.network?.tx_bps ?? null,
  };
}

export type UseDiagnosticsResult = ReturnType<typeof useQuery<Diagnostics>> & {
  history: DiagnosticsPoint[];
};

export function useDiagnostics(): UseDiagnosticsResult {
  const query = useQuery<Diagnostics>({
    queryKey: ['diagnostics'],
    queryFn: ({ signal }) => getDiagnostics(signal),
    refetchInterval: POLL_MS,
  });

  const [history, setHistory] = useState<DiagnosticsPoint[]>([]);
  const [seenAt, setSeenAt] = useState(0);

  // Append on fresh query data during render (React “adjust state when props
  // change” pattern). Keyed on dataUpdatedAt so unrelated re-renders cannot
  // duplicate points — same guarantee the plan required of the effect.
  if (query.data && query.dataUpdatedAt !== seenAt) {
    setSeenAt(query.dataUpdatedAt);
    setHistory((prev) => [...prev, toPoint(query.data, query.dataUpdatedAt)].slice(-MAX_SAMPLES));
  }

  return { ...query, history };
}
