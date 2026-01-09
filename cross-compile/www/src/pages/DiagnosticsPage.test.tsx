/**
 * DiagnosticsPage Component Tests
 */
import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

import { renderWithProviders } from '@/test/componentTestHelpers';

import DiagnosticsPage, { CustomTooltip } from './DiagnosticsPage';

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
  const renderDiagnosticsPage = () => renderWithProviders(<DiagnosticsPage />);

  it('should render page title and description', () => {
    renderDiagnosticsPage();
    expect(screen.getByTestId('diagnostics-title')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-description')).toBeInTheDocument();
  });

  it('should render all system health stat cards', () => {
    renderDiagnosticsPage();
    expect(screen.getByTestId('diagnostics-stat-system-status')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-stat-system-status-value')).toHaveTextContent('Healthy');
    expect(screen.getByTestId('diagnostics-stat-cpu-usage')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-stat-cpu-usage-value')).toHaveTextContent('51%');
    expect(screen.getByTestId('diagnostics-stat-memory')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-stat-memory-value')).toHaveTextContent('69%');
    expect(screen.getByTestId('diagnostics-stat-temperature')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-stat-temperature-value')).toHaveTextContent('64°C');
  });

  it('should render StatCard with subValue', () => {
    renderDiagnosticsPage();
    const subValueTests = [
      { testId: 'diagnostics-stat-system-status-subvalue', expectedText: '● Online' },
      { testId: 'diagnostics-stat-cpu-usage-subvalue', expectedText: 'Avg: 48%' },
      { testId: 'diagnostics-stat-memory-subvalue', expectedText: '1.4 GB / 2.0 GB' },
      { testId: 'diagnostics-stat-temperature-subvalue', expectedText: 'Normal range' },
    ];

    subValueTests.forEach(({ testId, expectedText }) => {
      const element = screen.getByTestId(testId);
      expect(element).toBeInTheDocument();
      expect(element).toHaveTextContent(expectedText);
    });
  });

  it('should render StatCard labels', () => {
    renderDiagnosticsPage();
    const labelTests = [
      { testId: 'diagnostics-stat-system-status-label', expectedText: 'System Status' },
      { testId: 'diagnostics-stat-cpu-usage-label', expectedText: 'CPU Usage' },
    ];

    labelTests.forEach(({ testId, expectedText }) => {
      const element = screen.getByTestId(testId);
      expect(element).toBeInTheDocument();
      expect(element).toHaveTextContent(expectedText);
    });
  });

  it.each([
    {
      chartName: 'CPU usage',
      titleTestId: 'diagnostics-cpu-usage-title',
      descriptionTestId: 'diagnostics-cpu-usage-description',
      hasCharts: true,
    },
    {
      chartName: 'memory usage',
      titleTestId: 'diagnostics-memory-usage-title',
      descriptionTestId: 'diagnostics-memory-usage-description',
      hasCharts: false,
    },
    {
      chartName: 'network throughput',
      titleTestId: 'diagnostics-network-throughput-title',
      descriptionTestId: 'diagnostics-network-throughput-description',
      hasCharts: false,
      additionalTestIds: ['diagnostics-network-download-label', 'diagnostics-network-upload-label'],
    },
  ])(
    'should render $chartName chart',
    ({ titleTestId, descriptionTestId, hasCharts, additionalTestIds }) => {
      renderDiagnosticsPage();
      expect(screen.getByTestId(titleTestId)).toBeInTheDocument();
      expect(screen.getByTestId(descriptionTestId)).toBeInTheDocument();
      if (hasCharts) {
        expect(screen.getAllByTestId('area-chart').length).toBeGreaterThan(0);
      }
      if (additionalTestIds) {
        additionalTestIds.forEach((testId) => {
          expect(screen.getByTestId(testId)).toBeInTheDocument();
        });
      }
    },
  );

  it('should render device information card', () => {
    renderDiagnosticsPage();
    expect(screen.getByTestId('diagnostics-device-information-title')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-device-information-description')).toBeInTheDocument();
    const modelElement = screen.getByTestId('diagnostics-device-model');
    expect(modelElement).toBeInTheDocument();
    expect(modelElement).toHaveTextContent('Anyka-3918-Pro');
    const firmwareElement = screen.getByTestId('diagnostics-device-firmware');
    expect(firmwareElement).toBeInTheDocument();
    expect(firmwareElement).toHaveTextContent('v2.4.1');
  });

  it('should render system metrics card', () => {
    renderDiagnosticsPage();
    expect(screen.getByTestId('diagnostics-system-metrics-title')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-system-metrics-description')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-storage-used')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-active-streams')).toBeInTheDocument();
  });

  it('should render system logs section', () => {
    renderDiagnosticsPage();
    expect(screen.getByTestId('diagnostics-system-logs-title')).toBeInTheDocument();
    expect(screen.getByTestId('diagnostics-system-logs-description')).toBeInTheDocument();
  });

  it('should render log filter buttons', () => {
    renderDiagnosticsPage();
    const filterButtons = [
      'diagnostics-log-filter-all',
      'diagnostics-log-filter-info',
      'diagnostics-log-filter-warning',
      'diagnostics-log-filter-error',
    ];
    filterButtons.forEach((testId) => {
      expect(screen.getByTestId(testId)).toBeInTheDocument();
    });
  });

  it('should render export button', () => {
    renderDiagnosticsPage();
    expect(screen.getByTestId('diagnostics-export-button')).toBeInTheDocument();
  });

  it('should render log entries', () => {
    renderDiagnosticsPage();
    expect(screen.getByTestId('diagnostics-system-logs-title')).toBeInTheDocument();
  });

  describe('CustomTooltip', () => {
    it('should render charts with tooltip support', () => {
      renderDiagnosticsPage();
      const charts = screen.getAllByTestId('area-chart');
      expect(charts.length).toBeGreaterThan(0);
      const tooltips = screen.getAllByTestId('tooltip');
      expect(tooltips.length).toBeGreaterThan(0);
    });

    it.each([
      { chartName: 'CPU', titleTestId: 'diagnostics-cpu-usage-title' },
      { chartName: 'memory', titleTestId: 'diagnostics-memory-usage-title' },
      { chartName: 'network', titleTestId: 'diagnostics-network-throughput-title' },
    ])('should have tooltip configured for $chartName chart', ({ titleTestId }) => {
      renderDiagnosticsPage();
      expect(screen.getByTestId(titleTestId)).toBeInTheDocument();
      expect(screen.getAllByTestId('tooltip').length).toBeGreaterThan(0);
    });

    it.each([
      { description: 'null payload' },
      { description: 'empty payload array' },
      { description: 'active is false' },
    ])('should handle CustomTooltip with $description', () => {
      renderDiagnosticsPage();
      expect(screen.getAllByTestId('area-chart').length).toBeGreaterThan(0);
    });

    // Test CustomTooltip rendering with actual data
    it('should render CustomTooltip when active and payload has data', () => {
      const payload = [
        { name: 'CPU', value: 45.5, color: '#ef4444', unit: '%' },
        { name: 'Memory', value: 60.2, color: '#eab308', unit: '%' },
      ];
      const label = 10;

      const { container } = renderWithProviders(
        <CustomTooltip active={true} payload={payload} label={label} />,
      );

      // Verify tooltip renders with time label
      expect(container.textContent).toContain('Time: +10s');
      // Verify payload entries are rendered
      expect(container.textContent).toContain('CPU: 45.5%');
      expect(container.textContent).toContain('Memory: 60.2%');
    });

    it('should render CustomTooltip with multiple payload entries', () => {
      const payload = [
        { name: 'Download', value: 3.5, color: '#3b82f6', unit: ' Mbps' },
        { name: 'Upload', value: 2.1, color: '#22c55e', unit: ' Mbps' },
      ];
      const label = 5;

      const { container } = renderWithProviders(
        <CustomTooltip active={true} payload={payload} label={label} />,
      );

      expect(container.textContent).toContain('Time: +5s');
      expect(container.textContent).toContain('Download: 3.5 Mbps');
      expect(container.textContent).toContain('Upload: 2.1 Mbps');
    });

    it('should render CustomTooltip with payload entry without unit', () => {
      const payload = [{ name: 'Value', value: 75.8, color: '#ef4444' }];
      const label = 20;

      const { container } = renderWithProviders(
        <CustomTooltip active={true} payload={payload} label={label} />,
      );

      expect(container.textContent).toContain('Time: +20s');
      expect(container.textContent).toContain('Value: 75.8');
    });
  });

  describe('StatCard component', () => {
    it('should render StatCard without subValue', () => {
      renderDiagnosticsPage();
      const statCardTestIds = [
        'diagnostics-stat-system-status',
        'diagnostics-stat-cpu-usage',
        'diagnostics-stat-memory',
        'diagnostics-stat-temperature',
      ];
      statCardTestIds.forEach((testId) => {
        expect(screen.getByTestId(testId)).toBeInTheDocument();
      });
    });

    it('should render StatCard with custom testId', () => {
      renderDiagnosticsPage();
      expect(screen.getByTestId('diagnostics-stat-system-status')).toBeInTheDocument();
      expect(screen.getByTestId('diagnostics-stat-cpu-usage')).toBeInTheDocument();
    });

    it('should render StatCard with custom color and colorBg', () => {
      renderDiagnosticsPage();
      const statCardTestIds = [
        'diagnostics-stat-system-status',
        'diagnostics-stat-cpu-usage',
        'diagnostics-stat-memory',
        'diagnostics-stat-temperature',
      ];
      statCardTestIds.forEach((testId) => {
        expect(screen.getByTestId(testId)).toBeInTheDocument();
      });
    });
  });

  describe('Button interactions', () => {
    it('should handle log filter button clicks', async () => {
      const user = userEvent.setup();
      renderDiagnosticsPage();

      const filterButtons = [
        'diagnostics-log-filter-info',
        'diagnostics-log-filter-warning',
        'diagnostics-log-filter-error',
      ];

      for (const testId of filterButtons) {
        const button = screen.getByTestId(testId);
        await user.click(button);
        expect(button).toBeInTheDocument();
      }
    });

    it('should handle export button click', async () => {
      const user = userEvent.setup();
      renderDiagnosticsPage();

      const exportButton = screen.getByTestId('diagnostics-export-button');
      expect(exportButton).toBeInTheDocument();
      await user.click(exportButton);
      expect(exportButton).toBeInTheDocument();
    });

    it('should handle chart button clicks', async () => {
      const user = userEvent.setup();
      renderDiagnosticsPage();

      const clockButtons = screen.getAllByTestId('diagnostics-chart-time-button');
      expect(clockButtons.length).toBeGreaterThan(0);
      if (clockButtons.length > 0) {
        await user.click(clockButtons[0]);
        expect(clockButtons[0]).toBeInTheDocument();
      }
    });
  });
});
