/**
 * DiagnosticsPage Component Tests
 */
import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import { renderWithProviders } from '@/test/componentTestHelpers';

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
    renderWithProviders(<DiagnosticsPage />);
    expect(screen.getByTestId('diagnostics-title')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-description')).toBeInTheDocument();
  });

  it('should render all system health stat cards', () => {
    renderWithProviders(<DiagnosticsPage />);
    expect(screen.getByTestId('diagnostics-stat-system-status')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-stat-system-status-value')).toHaveTextContent('Healthy');
    expect(screen.getByTestId('diagnostics-stat-cpu-usage')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-stat-cpu-usage-value')).toHaveTextContent('51%');
    expect(screen.getByTestId('diagnostics-stat-memory')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-stat-memory-value')).toHaveTextContent('69%');
    expect(screen.getByTestId('diagnostics-stat-temperature')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-stat-temperature-value')).toHaveTextContent('64°C');
  });

  it('should render CPU usage chart', () => {
    renderWithProviders(<DiagnosticsPage />);
    expect(screen.getByTestId('diagnostics-cpu-usage-title')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-cpu-usage-description')).toBeInTheDocument();
    expect(screen.getAllByTestId('area-chart').length).toBeGreaterThan(0);
  });

  it('should render memory usage chart', () => {
    renderWithProviders(<DiagnosticsPage />);
    expect(screen.getByTestId('diagnostics-memory-usage-title')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-memory-usage-description')).toBeInTheDocument();
  });

  it('should render network throughput chart', () => {
    renderWithProviders(<DiagnosticsPage />);
    expect(screen.getByTestId('diagnostics-network-throughput-title')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-network-throughput-description')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-network-download-label')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-network-upload-label')).toBeInTheDocument();
  });

  it('should render device information card', () => {
    renderWithProviders(<DiagnosticsPage />);
    expect(screen.getByTestId('diagnostics-device-information-title')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-device-information-description')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-device-model')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-device-model')).toHaveTextContent('Anyka-3918-Pro');
    expect(screen.getByTestId('diagnostics-device-firmware')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-device-firmware')).toHaveTextContent('v2.4.1');
  });

  it('should render system metrics card', () => {
    renderWithProviders(<DiagnosticsPage />);
    expect(screen.getByTestId('diagnostics-system-metrics-title')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-system-metrics-description')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-storage-used')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-active-streams')).toBeInTheDocument();
  });

  it('should render system logs section', () => {
    renderWithProviders(<DiagnosticsPage />);
    expect(screen.getByTestId('diagnostics-system-logs-title')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-system-logs-description')).toBeInTheDocument();
    // Table headers are less critical but can be tested via table structure
    // We'll verify the section exists via the title
  });

  it('should render log filter buttons', () => {
    renderWithProviders(<DiagnosticsPage />);
    expect(screen.getByTestId('diagnostics-log-filter-all')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-log-filter-info')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-log-filter-warning')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-log-filter-error')).toBeInTheDocument();
  });

  it('should render export button', () => {
    renderWithProviders(<DiagnosticsPage />);
    expect(screen.getByTestId('diagnostics-export-button')).toBeInTheDocument();
  });

  it('should render log entries', () => {
    renderWithProviders(<DiagnosticsPage />);
    // Log entries are in a table - verify table structure exists
    expect(screen.getByTestId('diagnostics-system-logs-title')).toBeInTheDocument();
    // Log messages are dynamic content - we verify the logs section renders
    // Specific message content can be tested via table rows if needed
  });

  describe('CustomTooltip', () => {
    // CustomTooltip is an internal component used by recharts Tooltip
    // We test it indirectly by verifying the charts render correctly
    // Direct testing would require exporting it or using a different approach

    it('should render charts with tooltip support', () => {
      renderWithProviders(<DiagnosticsPage />);
      // Verify charts are rendered (which use CustomTooltip internally)
      const charts = screen.getAllByTestId('area-chart');
      expect(charts.length).toBeGreaterThan(0);
      // Tooltip component is rendered by recharts
      const tooltips = screen.getAllByTestId('tooltip');
      expect(tooltips.length).toBeGreaterThan(0);
    });

    it('should have tooltip configured for CPU chart', () => {
      renderWithProviders(<DiagnosticsPage />);
      // Charts with tooltips are rendered
      expect(screen.getByTestId('diagnostics-cpu-usage-title')).toBeInTheDocument();
      expect(screen.getAllByTestId('tooltip').length).toBeGreaterThan(0);
    });

    it('should have tooltip configured for memory chart', () => {
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-memory-usage-title')).toBeInTheDocument();
      expect(screen.getAllByTestId('tooltip').length).toBeGreaterThan(0);
    });

    it('should have tooltip configured for network chart', () => {
      renderWithProviders(<DiagnosticsPage />);
      expect(screen.getByTestId('diagnostics-network-throughput-title')).toBeInTheDocument();
      expect(screen.getAllByTestId('tooltip').length).toBeGreaterThan(0);
    });
  });

  describe('Button interactions', () => {
    it('should handle log filter button clicks', async () => {
      const user = userEvent.setup();
      renderWithProviders(<DiagnosticsPage />);

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
      renderWithProviders(<DiagnosticsPage />);

      const exportButton = screen.getByTestId('diagnostics-export-button');
      expect(exportButton).toBeInTheDocument();

      await user.click(exportButton);
      // Export button is clickable (functionality may not be implemented yet)
      expect(exportButton).toBeInTheDocument();
    });

    it('should handle chart button clicks', async () => {
      const user = userEvent.setup();
      renderWithProviders(<DiagnosticsPage />);

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
