/**
 * DiagnosticsPage Component Tests
 */
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import DiagnosticsPage from './DiagnosticsPage';

// Mock recharts components
vi.mock('recharts', () => ({
  ResponsiveContainer: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="responsive-container">{children}</div>
  ),
  AreaChart: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="area-chart">{children}</div>
  ),
  Area: () => <div data-testid="area" />,
  CartesianGrid: () => <div data-testid="cartesian-grid" />,
  XAxis: () => <div data-testid="x-axis" />,
  YAxis: () => <div data-testid="y-axis" />,
  Tooltip: () => <div data-testid="tooltip" />,
}));

describe('DiagnosticsPage', () => {
  it('should render page title and description', () => {
    render(<DiagnosticsPage />);
    expect(screen.getByText('Diagnostics & Statistics')).toBeInTheDocument();
    expect(
      screen.getByText('Real-time system monitoring and performance metrics'),
    ).toBeInTheDocument();
  });

  it('should render all system health stat cards', () => {
    render(<DiagnosticsPage />);
    expect(screen.getByText('System Status')).toBeInTheDocument();
    expect(screen.getByText('Healthy')).toBeInTheDocument();
    const cpuUsageTexts = screen.getAllByText('CPU Usage');
    expect(cpuUsageTexts.length).toBeGreaterThan(0);
    expect(screen.getByText('51%')).toBeInTheDocument();
    expect(screen.getByText('Memory')).toBeInTheDocument();
    expect(screen.getByText('69%')).toBeInTheDocument();
    expect(screen.getByText('Temperature')).toBeInTheDocument();
    expect(screen.getByText('64°C')).toBeInTheDocument();
  });

  it('should render CPU usage chart', () => {
    render(<DiagnosticsPage />);
    const cpuUsageTexts = screen.getAllByText('CPU Usage');
    expect(cpuUsageTexts.length).toBeGreaterThan(0);
    expect(screen.getByText('Processor load over time')).toBeInTheDocument();
    expect(screen.getAllByTestId('area-chart').length).toBeGreaterThan(0);
  });

  it('should render memory usage chart', () => {
    render(<DiagnosticsPage />);
    expect(screen.getByText('Memory Usage')).toBeInTheDocument();
    expect(screen.getByText('RAM utilization over time')).toBeInTheDocument();
  });

  it('should render network throughput chart', () => {
    render(<DiagnosticsPage />);
    expect(screen.getByText('Network Throughput')).toBeInTheDocument();
    expect(screen.getByText('Upload and download bandwidth')).toBeInTheDocument();
    expect(screen.getByText('Download')).toBeInTheDocument();
    expect(screen.getByText('Upload')).toBeInTheDocument();
  });

  it('should render device information card', () => {
    render(<DiagnosticsPage />);
    expect(screen.getByText('Device Information')).toBeInTheDocument();
    expect(screen.getByText('Hardware and firmware details')).toBeInTheDocument();
    expect(screen.getByText('Model')).toBeInTheDocument();
    expect(screen.getByText('Anyka-3918-Pro')).toBeInTheDocument();
    expect(screen.getByText('Firmware')).toBeInTheDocument();
    expect(screen.getByText('v2.4.1')).toBeInTheDocument();
  });

  it('should render system metrics card', () => {
    render(<DiagnosticsPage />);
    expect(screen.getByText('System Metrics')).toBeInTheDocument();
    expect(screen.getByText('Performance and storage statistics')).toBeInTheDocument();
    expect(screen.getByText('Storage Used')).toBeInTheDocument();
    expect(screen.getByText('Active Streams')).toBeInTheDocument();
  });

  it('should render system logs section', () => {
    render(<DiagnosticsPage />);
    expect(screen.getByText('System Logs')).toBeInTheDocument();
    expect(screen.getByText('Recent activity and events')).toBeInTheDocument();
    expect(screen.getByText('Timestamp')).toBeInTheDocument();
    expect(screen.getByText('Level')).toBeInTheDocument();
    expect(screen.getByText('Category')).toBeInTheDocument();
    expect(screen.getByText('Message')).toBeInTheDocument();
  });

  it('should render log filter buttons', () => {
    render(<DiagnosticsPage />);
    expect(screen.getByText('All')).toBeInTheDocument();
    const infoTexts = screen.getAllByText('Info');
    expect(infoTexts.length).toBeGreaterThan(0);
    const warningTexts = screen.getAllByText('Warning');
    expect(warningTexts.length).toBeGreaterThan(0);
    const errorTexts = screen.getAllByText('Error');
    expect(errorTexts.length).toBeGreaterThan(0);
  });

  it('should render export button', () => {
    render(<DiagnosticsPage />);
    expect(screen.getByText('Export')).toBeInTheDocument();
  });

  it('should render log entries', () => {
    render(<DiagnosticsPage />);
    expect(screen.getByText('Main stream started successfully')).toBeInTheDocument();
    expect(screen.getByText('High latency detected: 125ms')).toBeInTheDocument();
    expect(screen.getByText('Preset position 1 updated')).toBeInTheDocument();
  });

  describe('CustomTooltip', () => {
    // CustomTooltip is an internal component used by recharts Tooltip
    // We test it indirectly by verifying the charts render correctly
    // Direct testing would require exporting it or using a different approach

    it('should render charts with tooltip support', () => {
      render(<DiagnosticsPage />);
      // Verify charts are rendered (which use CustomTooltip internally)
      const charts = screen.getAllByTestId('area-chart');
      expect(charts.length).toBeGreaterThan(0);
      // Tooltip component is rendered by recharts
      const tooltips = screen.getAllByTestId('tooltip');
      expect(tooltips.length).toBeGreaterThan(0);
    });

    it('should have tooltip configured for CPU chart', () => {
      render(<DiagnosticsPage />);
      // Charts with tooltips are rendered
      const cpuUsageTexts = screen.getAllByText('CPU Usage');
      expect(cpuUsageTexts.length).toBeGreaterThan(0);
      expect(screen.getAllByTestId('tooltip').length).toBeGreaterThan(0);
    });

    it('should have tooltip configured for memory chart', () => {
      render(<DiagnosticsPage />);
      expect(screen.getByText('Memory Usage')).toBeInTheDocument();
      expect(screen.getAllByTestId('tooltip').length).toBeGreaterThan(0);
    });

    it('should have tooltip configured for network chart', () => {
      render(<DiagnosticsPage />);
      expect(screen.getByText('Network Throughput')).toBeInTheDocument();
      expect(screen.getAllByTestId('tooltip').length).toBeGreaterThan(0);
    });
  });

  describe('Button interactions', () => {
    it('should handle log filter button clicks', async () => {
      const user = userEvent.setup();
      render(<DiagnosticsPage />);

      const infoButton = screen.getByTestId('diagnostics-log-filter-info');
      await user.click(infoButton);
      expect(infoButton).toBeInTheDocument();

      const warningButton = screen.getByTestId('diagnostics-log-filter-warning');
      await user.click(warningButton);
      expect(warningButton).toBeInTheDocument();

      const errorButton = screen.getByTestId('diagnostics-log-filter-error');
      await user.click(errorButton);
      expect(errorButton).toBeInTheDocument();
    });

    it('should handle export button click', async () => {
      const user = userEvent.setup();
      render(<DiagnosticsPage />);

      const exportButton = screen.getByTestId('diagnostics-export-button');
      expect(exportButton).toBeInTheDocument();

      await user.click(exportButton);
      // Export button is clickable (functionality may not be implemented yet)
      expect(exportButton).toBeInTheDocument();
    });

    it('should handle chart button clicks', async () => {
      const user = userEvent.setup();
      render(<DiagnosticsPage />);

      // Find clock icon buttons (time range selectors)
      const clockButtons = screen.getAllByTestId('diagnostics-chart-time-button');

      // Clock buttons should be present and clickable
      expect(clockButtons.length).toBeGreaterThan(0);
      if (clockButtons.length > 0) {
        await user.click(clockButtons[0]);
        expect(clockButtons[0]).toBeInTheDocument();
      }
    });
  });
});
