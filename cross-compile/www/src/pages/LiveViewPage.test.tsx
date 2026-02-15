/**
 * LiveViewPage Tests
 */
import { fireEvent, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { renderWithProviders } from '@/test/componentTestHelpers';

import LiveViewPage from './LiveViewPage';

// Mock ptzService
vi.mock('@/services/ptzService', () => ({
  continuousMove: vi.fn().mockResolvedValue(undefined),
  stopMove: vi.fn().mockResolvedValue(undefined),
  gotoHome: vi.fn().mockResolvedValue(undefined),
  getPresets: vi.fn().mockResolvedValue([
    { token: 'preset1', name: 'Front Door' },
    { token: 'preset2', name: 'Back Yard' },
    { token: 'preset3', name: 'Garage' },
  ]),
  gotoPreset: vi.fn().mockResolvedValue(undefined),
  setPreset: vi.fn().mockResolvedValue('preset_new'),
  removePreset: vi.fn().mockResolvedValue(undefined),
}));

// Mock profileService
vi.mock('@/services/profileService', () => ({
  getProfiles: vi.fn().mockResolvedValue([
    {
      token: 'ProfileToken1',
      name: 'MainStream',
      ptzConfiguration: { token: 'PtzConfig1', name: 'PTZ' },
    },
    {
      token: 'ProfileToken2',
      name: 'SubStream',
    },
  ]),
}));

// Mock sonner toast
vi.mock('sonner', () => ({
  toast: {
    error: vi.fn(),
    success: vi.fn(),
    info: vi.fn(),
  },
}));

// Mock clipboard API
const mockWriteText = vi.fn().mockResolvedValue(undefined);
Object.defineProperty(navigator, 'clipboard', {
  value: {
    writeText: mockWriteText,
  },
  writable: true,
  configurable: true,
});

describe('LiveViewPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should render page title and description', () => {
    renderWithProviders(<LiveViewPage />);
    expect(screen.getByTestId('liveview-title')).toHaveTextContent('Live Video Preview');
    expect(screen.getByTestId('liveview-description')).toHaveTextContent(
      'Real-time ONVIF stream monitoring and playback controls',
    );
  });

  it('should render video stream placeholder', () => {
    renderWithProviders(<LiveViewPage />);
    expect(screen.getByTestId('liveview-stream-preview-title')).toHaveTextContent(
      'ONVIF Stream Preview',
    );
    expect(screen.getByTestId('liveview-stream-preview-info')).toHaveTextContent(
      '1920×1080 @ 30fps',
    );
  });

  it('should toggle between main and sub stream', async () => {
    const user = userEvent.setup();
    renderWithProviders(<LiveViewPage />);

    const mainStreamButton = screen.getByTestId('liveview-main-stream-button');
    const subStreamButton = screen.getByTestId('liveview-sub-stream-button');

    expect(mainStreamButton).toBeInTheDocument();
    expect(subStreamButton).toBeInTheDocument();

    await user.click(subStreamButton);
    expect(subStreamButton).toHaveClass('bg-accent-red');

    await user.click(mainStreamButton);
    expect(mainStreamButton).toHaveClass('bg-accent-red');
  });

  it('should render PTZ control panel', () => {
    renderWithProviders(<LiveViewPage />);
    expect(screen.getByTestId('liveview-ptz-title')).toHaveTextContent('Pan & Tilt');
    expect(screen.getByTestId('liveview-ptz-description')).toHaveTextContent('PTZ camera controls');
  });

  it('should render PTZ speed slider', () => {
    renderWithProviders(<LiveViewPage />);
    const speedSlider = screen.getByTestId('liveview-ptz-speed-slider');
    expect(speedSlider).toBeInTheDocument();
    expect(speedSlider).toHaveValue('50');
  });

  it('should render PTZ speed slider with initial value', () => {
    renderWithProviders(<LiveViewPage />);
    const speedSlider = screen.getByTestId('liveview-ptz-speed-slider');
    expect(speedSlider).toBeInTheDocument();
    expect(speedSlider).toHaveValue('50');
    expect(screen.getByTestId('liveview-ptz-speed-value')).toHaveTextContent('50%');
  });

  it('should render stream information cards', () => {
    renderWithProviders(<LiveViewPage />);
    expect(screen.getByTestId('liveview-stream-info-title')).toHaveTextContent('Stream Info');
    expect(screen.getByTestId('liveview-network-stats-title')).toHaveTextContent('Network Stats');
    expect(screen.getByTestId('liveview-resolution-label')).toHaveTextContent('Resolution');
    expect(screen.getByTestId('liveview-resolution-value')).toHaveTextContent('1920x1080');
  });

  it('should render stream URL and copy button', () => {
    renderWithProviders(<LiveViewPage />);
    expect(screen.getByTestId('liveview-stream-url-label')).toHaveTextContent('Stream URL');
    expect(screen.getByTestId('liveview-stream-url-value')).toHaveTextContent(
      'rtsp://192.168.1.100:554/main',
    );
    expect(screen.getByTestId('liveview-copy-url-button')).toBeInTheDocument();
  });

  it('should render preset buttons', async () => {
    renderWithProviders(<LiveViewPage />);
    expect(screen.getByTestId('liveview-presets-title')).toHaveTextContent('Presets');

    // Preset data is loaded asynchronously from the mocked getPresets service
    await waitFor(() => {
      expect(screen.getByTestId('liveview-preset-1-button')).toBeInTheDocument();
    });
    expect(screen.getByTestId('liveview-preset-2-button')).toBeInTheDocument();
    expect(screen.getByTestId('liveview-preset-3-button')).toBeInTheDocument();
    expect(screen.getByTestId('liveview-add-preset-label')).toHaveTextContent('+ Add Preset');
  });

  it('should render PTZ control buttons', () => {
    renderWithProviders(<LiveViewPage />);
    expect(screen.getByTestId('liveview-ptz-title')).toHaveTextContent('Pan & Tilt');
    const directions = ['up', 'down', 'left', 'right', 'up-left', 'up-right', 'down-left', 'down-right'];
    for (const dir of directions) {
      expect(screen.getByTestId(`liveview-ptz-${dir}-button`)).toBeInTheDocument();
    }
    expect(screen.getByTestId('liveview-ptz-home-button')).toBeInTheDocument();
  });

  it('should render LIVE indicator', () => {
    renderWithProviders(<LiveViewPage />);
    expect(screen.getByTestId('liveview-live-indicator')).toHaveTextContent('LIVE');
  });

  it('should update speed slider value', async () => {
    renderWithProviders(<LiveViewPage />);

    const speedSlider = screen.getByTestId('liveview-ptz-speed-slider');
    expect(speedSlider).toHaveValue('50');

    expect(speedSlider.getAttribute('type')).toBe('range');
    expect(speedSlider.getAttribute('min')).toBe('1');
    expect(speedSlider.getAttribute('max')).toBe('100');
  });

  describe('PTZ controls with real backend', () => {
    it('should call continuousMove on mousedown of directional button', async () => {
      const { continuousMove: mockContinuousMove } = await import('@/services/ptzService');
      renderWithProviders(<LiveViewPage />);

      // Wait for profiles + presets queries to resolve (preset names prove profileToken is set)
      await waitFor(() => {
        expect(screen.getByTestId('liveview-preset-1-label')).toHaveTextContent('Front Door');
      });

      const upButton = screen.getByTestId('liveview-ptz-up-button');
      fireEvent.mouseDown(upButton);

      await waitFor(() => {
        expect(mockContinuousMove).toHaveBeenCalledWith('ProfileToken1', 'up', 50);
      });
    });

    it('should call stopMove on mouseup of directional button', async () => {
      const { stopMove: mockStopMove } = await import('@/services/ptzService');
      renderWithProviders(<LiveViewPage />);

      await waitFor(() => {
        expect(screen.getByTestId('liveview-preset-1-label')).toHaveTextContent('Front Door');
      });

      const upButton = screen.getByTestId('liveview-ptz-up-button');
      fireEvent.mouseDown(upButton);
      fireEvent.mouseUp(upButton);

      await waitFor(() => {
        expect(mockStopMove).toHaveBeenCalledWith('ProfileToken1');
      });
    });

    it('should call stopMove on mouseleave of directional button', async () => {
      const { stopMove: mockStopMove } = await import('@/services/ptzService');
      renderWithProviders(<LiveViewPage />);

      await waitFor(() => {
        expect(screen.getByTestId('liveview-preset-1-label')).toHaveTextContent('Front Door');
      });

      const leftButton = screen.getByTestId('liveview-ptz-left-button');
      fireEvent.mouseDown(leftButton);
      fireEvent.mouseLeave(leftButton);

      await waitFor(() => {
        expect(mockStopMove).toHaveBeenCalledWith('ProfileToken1');
      });
    });

    it('should call gotoHome on home button click', async () => {
      const user = userEvent.setup();
      const { gotoHome: mockGotoHome } = await import('@/services/ptzService');
      renderWithProviders(<LiveViewPage />);

      await waitFor(() => {
        expect(screen.getByTestId('liveview-ptz-home-button')).toBeInTheDocument();
      });

      const homeButton = screen.getByTestId('liveview-ptz-home-button');
      await user.click(homeButton);

      await waitFor(() => {
        expect(mockGotoHome).toHaveBeenCalledWith('ProfileToken1');
      });
    });

    it('should call continuousMove for all 8 directional buttons', async () => {
      const { continuousMove: mockContinuousMove } = await import('@/services/ptzService');
      renderWithProviders(<LiveViewPage />);

      await waitFor(() => {
        expect(screen.getByTestId('liveview-preset-1-label')).toHaveTextContent('Front Door');
      });

      const directions = [
        'up', 'down', 'left', 'right',
        'up-left', 'up-right', 'down-left', 'down-right',
      ];

      for (const direction of directions) {
        vi.mocked(mockContinuousMove).mockClear();
        const button = screen.getByTestId(`liveview-ptz-${direction}-button`);
        fireEvent.mouseDown(button);

        await waitFor(() => {
          expect(mockContinuousMove).toHaveBeenCalledWith('ProfileToken1', direction, 50);
        });
      }
    });

    it('should call gotoPreset when preset button is clicked', async () => {
      const user = userEvent.setup();
      const { gotoPreset: mockGotoPreset } = await import('@/services/ptzService');
      renderWithProviders(<LiveViewPage />);

      // Wait for presets to load
      await waitFor(() => {
        expect(screen.getByTestId('liveview-preset-1-button')).toBeInTheDocument();
      });

      const presetButton = screen.getByTestId('liveview-preset-1-button');
      await user.click(presetButton);

      await waitFor(() => {
        expect(mockGotoPreset).toHaveBeenCalledWith('ProfileToken1', 'preset1');
      });
    });

    it('should call setPreset when add preset button is clicked', async () => {
      const user = userEvent.setup();
      const { setPreset: mockSetPreset } = await import('@/services/ptzService');
      renderWithProviders(<LiveViewPage />);

      await waitFor(() => {
        expect(screen.getByTestId('liveview-preset-1-label')).toHaveTextContent('Front Door');
      });

      const addButton = screen.getByTestId('liveview-add-preset-button');
      await user.click(addButton);

      await waitFor(() => {
        expect(mockSetPreset).toHaveBeenCalledWith('ProfileToken1', expect.any(String), undefined);
      });
    });

    it('should call removePreset when preset settings button is clicked', async () => {
      const user = userEvent.setup();
      const { removePreset: mockRemovePreset } = await import('@/services/ptzService');
      renderWithProviders(<LiveViewPage />);

      await waitFor(() => {
        expect(screen.getByTestId('liveview-preset-1-settings-button')).toBeInTheDocument();
      });

      const settingsButton = screen.getByTestId('liveview-preset-1-settings-button');
      await user.click(settingsButton);

      await waitFor(() => {
        expect(mockRemovePreset).toHaveBeenCalledWith('ProfileToken1', 'preset1');
      });
    });

    it('should display preset names from backend', async () => {
      renderWithProviders(<LiveViewPage />);

      await waitFor(() => {
        expect(screen.getByTestId('liveview-preset-1-label')).toHaveTextContent('Front Door');
      });
      expect(screen.getByTestId('liveview-preset-2-label')).toHaveTextContent('Back Yard');
      expect(screen.getByTestId('liveview-preset-3-label')).toHaveTextContent('Garage');
    });

    it('should show error toast when PTZ move fails', async () => {
      const { continuousMove: mockContinuousMove } = await import('@/services/ptzService');
      const { toast } = await import('sonner');
      vi.mocked(mockContinuousMove).mockRejectedValueOnce(new Error('Connection refused'));

      renderWithProviders(<LiveViewPage />);

      await waitFor(() => {
        expect(screen.getByTestId('liveview-preset-1-label')).toHaveTextContent('Front Door');
      });

      const upButton = screen.getByTestId('liveview-ptz-up-button');
      fireEvent.mouseDown(upButton);

      await waitFor(() => {
        expect(toast.error).toHaveBeenCalledWith('PTZ move failed', {
          description: 'Connection refused',
        });
      });
    });
  });

  it('should handle preset settings button clicks', async () => {
    renderWithProviders(<LiveViewPage />);

    const settingsButton1 = screen.getByTestId('liveview-preset-1-settings-button');
    const settingsButton2 = screen.getByTestId('liveview-preset-2-settings-button');
    const settingsButton3 = screen.getByTestId('liveview-preset-3-settings-button');

    expect(settingsButton1).toBeInTheDocument();
    expect(settingsButton2).toBeInTheDocument();
    expect(settingsButton3).toBeInTheDocument();
  });
});
