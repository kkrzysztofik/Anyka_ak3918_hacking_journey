import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { Sparkline } from './Sparkline';

const points = [
  { time: 0, value: 10 },
  { time: 1, value: 50 },
  { time: 2, value: 30 },
];

describe('Sparkline', () => {
  it('test_sparkline_single_series_renders_one_area_path', () => {
    render(
      <Sparkline
        data={points}
        series={[{ key: 'value', label: 'CPU', color: '#ef4444', unit: '%' }]}
        domain={[0, 100]}
      />,
    );
    expect(screen.getAllByTestId('sparkline-area')).toHaveLength(1);
  });

  it('test_sparkline_two_series_renders_two_area_paths', () => {
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

  it('test_sparkline_with_data_produces_path_without_nan_coordinates', () => {
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

  it('test_sparkline_fixed_domain_honours_bounds_instead_of_auto_scaling', () => {
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

  it('test_sparkline_empty_data_renders_nothing_without_crashing', () => {
    render(<Sparkline data={[]} series={[{ key: 'value', label: 'CPU', color: '#ef4444' }]} />);
    expect(screen.queryAllByTestId('sparkline-area')).toHaveLength(0);
  });

  it('test_sparkline_with_units_announces_latest_value_per_series', () => {
    render(
      <Sparkline
        data={[
          { time: 0, upload: 2, download: 4 },
          { time: 1, upload: 3, download: 5 },
        ]}
        series={[
          { key: 'download', label: 'Download', color: '#3b82f6', unit: ' Mbps' },
          { key: 'upload', label: 'Upload', color: '#22c55e', unit: ' Mbps' },
        ]}
      />,
    );
    expect(screen.getByRole('img')).toHaveAttribute('aria-label', 'Download 5 Mbps, Upload 3 Mbps');
  });

  it('test_sparkline_without_unit_falls_back_to_bare_label', () => {
    render(<Sparkline data={points} series={[{ key: 'value', label: 'CPU', color: '#ef4444' }]} />);
    expect(screen.getByRole('img')).toHaveAttribute('aria-label', 'CPU 30');
  });

  it('test_sparkline_with_data_renders_legend_entry_per_series', () => {
    render(
      <Sparkline
        data={[
          { time: 0, upload: 2, download: 4 },
          { time: 1, upload: 3, download: 5 },
        ]}
        series={[
          { key: 'download', label: 'Download', color: '#3b82f6', unit: ' Mbps' },
          { key: 'upload', label: 'Upload', color: '#22c55e', unit: ' Mbps' },
        ]}
      />,
    );
    expect(screen.getByTestId('sparkline-legend')).toBeInTheDocument();
    expect(screen.getByTestId('sparkline-legend-download')).toHaveTextContent('Download');
    expect(screen.getByTestId('sparkline-legend-download-value')).toHaveTextContent('5 Mbps');
    expect(screen.getByTestId('sparkline-legend-upload')).toHaveTextContent('Upload');
    expect(screen.getByTestId('sparkline-legend-upload-value')).toHaveTextContent('3 Mbps');
  });

  it('test_sparkline_empty_data_omits_legend', () => {
    render(<Sparkline data={[]} series={[{ key: 'value', label: 'CPU', color: '#ef4444' }]} />);
    expect(screen.queryByTestId('sparkline-legend')).not.toBeInTheDocument();
  });
});
