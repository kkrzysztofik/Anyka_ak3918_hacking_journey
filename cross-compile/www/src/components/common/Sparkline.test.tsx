import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { Sparkline } from './Sparkline';

const points = [
  { time: 0, value: 10 },
  { time: 1, value: 50 },
  { time: 2, value: 30 },
];

describe('Sparkline', () => {
  it('renders one filled area path per series', () => {
    render(
      <Sparkline
        data={points}
        series={[{ key: 'value', label: 'CPU', color: '#ef4444', unit: '%' }]}
        domain={[0, 100]}
      />,
    );
    expect(screen.getAllByTestId('sparkline-area')).toHaveLength(1);
  });

  it('renders an area per series when given two series', () => {
    const net = [
      { time: 0, upload: 2, download: 4 },
      { time: 1, upload: 3, download: 5 },
    ];
    render(
      <Sparkline
        data={net}
        series={[
          { key: 'download', label: 'Download', color: '#3b82f6', unit: ' Mbps' },
          { key: 'upload', label: 'Upload', color: '#22c55e', unit: ' Mbps' },
        ]}
      />,
    );
    expect(screen.getAllByTestId('sparkline-area')).toHaveLength(2);
  });

  it('produces a well-formed path with no NaN coordinates', () => {
    render(
      <Sparkline
        data={points}
        series={[{ key: 'value', label: 'CPU', color: '#ef4444' }]}
        domain={[0, 100]}
      />,
    );
    const d = screen.getAllByTestId('sparkline-area')[0].getAttribute('d');
    expect(d).toBeTruthy();
    expect(d).not.toContain('NaN');
  });

  it('honours a fixed domain rather than auto-scaling to the data', () => {
    // Values 10..50 under domain [0,100] must not touch the top of the box.
    render(
      <Sparkline
        data={points}
        series={[{ key: 'value', label: 'CPU', color: '#ef4444' }]}
        domain={[0, 100]}
      />,
    );
    const d = screen.getAllByTestId('sparkline-area')[0].getAttribute('d') ?? '';
    const ys = [...d.matchAll(/[ML](?:[\d.]+),([\d.]+)/g)].map((m) => Number(m[1]));
    expect(Math.min(...ys)).toBeGreaterThan(0);
  });

  it('renders nothing rather than crashing on empty data', () => {
    render(<Sparkline data={[]} series={[{ key: 'value', label: 'CPU', color: '#ef4444' }]} />);
    expect(screen.queryAllByTestId('sparkline-area')).toHaveLength(0);
  });
});
