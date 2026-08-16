/**
 * LiveViewPage Tests
 */
import { fireEvent, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { renderWithProviders } from '@/test/componentTestHelpers';
import { testDialogClosed, testDialogOpen } from '@/test/dialogTestHelpers';

import LiveViewPage from './LiveViewPage';

const mockPlayer = {
  on: vi.fn(),
  attachMediaElement: vi.fn(),
  load: vi.fn(),
  destroy: vi.fn(),
};
const createPlayer = vi.fn((_media?: unknown, _config?: unknown) => mockPlayer);

vi.mock('mpegts.js', () => ({
  default: {
    isSupported: () => true,
    createPlayer: (media: unknown, config?: unknown) => createPlayer(media, config),
    Events: {
      MEDIA_INFO: 'media_info',
      STATISTICS_INFO: 'statistics_info',
      ERROR: 'error',
    },
  },
}));

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
    // jsdom resets navigator.clipboard between tests, so (re)install the
    // module-level mock so the copy test and any later one share the same mock.
    Object.defineProperty(navigator, 'clipboard', {
      value: { writeText: mockWriteText },
      writable: true,
      configurable: true,
    });
  });

  /**
   * The player's event handlers are registered asynchronously (after the
   * mocked dynamic import resolves), so wait until the specific `event`
   * registration exists before retrieving the handler to invoke.
   */
  async function waitForPlayerHandler(event: string): Promise<(...args: unknown[]) => void> {
    let handler: ((...args: unknown[]) => void) | undefined;
    await waitFor(() => {
      handler = mockPlayer.on.mock.calls.find((c) => c[0] === event)?.[1];
      expect(handler).toBeDefined();
    });
    return handler as (...args: unknown[]) => void;
  }

  it('should render page title and description', () => {
    renderWithProviders(<LiveViewPage />);
    expect(screen.getByTestId('liveview-title')).toHaveTextContent('Live Video Preview');
    expect(screen.getByTestId('liveview-description')).toHaveTextContent(
      'Real-time ONVIF stream monitoring and playback controls',
    );
  });

  it('should render the live video element', async () => {
    renderWithProviders(<LiveViewPage />);
    expect(await screen.findByTestId('liveview-video')).toBeInTheDocument();
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

  it('should disable PTZ controls when no profile has a PTZ configuration', async () => {
    const { getProfiles } = await import('@/services/profileService');
    vi.mocked(getProfiles).mockResolvedValueOnce([
      { token: 'ProfileToken1', name: 'MainStream' },
      { token: 'ProfileToken2', name: 'SubStream' },
    ]);
    renderWithProviders(<LiveViewPage />);
    await waitFor(() => {
      expect(screen.getByTestId('liveview-ptz-fieldset')).toBeDisabled();
    });
    expect(screen.getByTestId('liveview-ptz-disabled-note')).toHaveTextContent('PTZ is disabled');
  });

  it('should not show the PTZ disabled note while profiles are still loading', async () => {
    const { getProfiles } = await import('@/services/profileService');
    vi.mocked(getProfiles).mockReturnValueOnce(new Promise(() => {})); // never resolves
    renderWithProviders(<LiveViewPage />);
    await screen.findByTestId('liveview-ptz-title');
    expect(screen.queryByTestId('liveview-ptz-disabled-note')).not.toBeInTheDocument();
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
    expect(screen.getByTestId('liveview-resolution-value')).toHaveTextContent('—');
  });

  it('should render stream URL and copy button', () => {
    renderWithProviders(<LiveViewPage />);
    expect(screen.getByTestId('liveview-stream-url-label')).toHaveTextContent('Stream URL');
    expect(screen.getByTestId('liveview-stream-url-value')).toHaveTextContent('/live/main.flv');
    expect(screen.getByTestId('liveview-copy-url-button')).toBeInTheDocument();
  });

  it('should render preset buttons', async () => {
    renderWithProviders(<LiveViewPage />);
    expect(screen.getByTestId('liveview-presets-title')).toHaveTextContent('Presets');

    // Preset data is loaded asynchronously from the mocked getPresets service
    await waitFor(() => {
      expect(screen.getByTestId('liveview-preset-preset1-button')).toBeInTheDocument();
    });
    expect(screen.getByTestId('liveview-preset-preset2-button')).toBeInTheDocument();
    expect(screen.getByTestId('liveview-preset-preset3-button')).toBeInTheDocument();
    expect(screen.getByTestId('liveview-add-preset-label')).toHaveTextContent(
      '+ Save current position',
    );
  });

  it('renders a row for every preset the device reports, not just three', async () => {
    const { getPresets } = await import('@/services/ptzService');
    vi.mocked(getPresets).mockResolvedValueOnce([
      { token: 'preset1', name: 'Front Door' },
      { token: 'preset2', name: 'Back Yard' },
      { token: 'preset3', name: 'Garage' },
      { token: 'preset4', name: 'Driveway' },
    ]);
    renderWithProviders(<LiveViewPage />);

    expect(await screen.findByTestId('liveview-preset-preset4-button')).toBeInTheDocument();
    for (const token of ['preset1', 'preset2', 'preset3']) {
      expect(screen.getByTestId(`liveview-preset-${token}-button`)).toBeInTheDocument();
    }
  });

  it('shows an empty state and no placeholder rows when there are no presets', async () => {
    const { getPresets } = await import('@/services/ptzService');
    vi.mocked(getPresets).mockResolvedValueOnce([]);
    renderWithProviders(<LiveViewPage />);

    expect(await screen.findByTestId('liveview-presets-empty')).toBeInTheDocument();
    expect(screen.queryByTestId('liveview-preset-preset1-button')).not.toBeInTheDocument();
    // The old fixed slots rendered "Preset 1" rows that pointed at nothing.
    expect(screen.queryByText('Preset 1')).not.toBeInTheDocument();
  });

  it('should render PTZ control buttons', () => {
    renderWithProviders(<LiveViewPage />);
    expect(screen.getByTestId('liveview-ptz-title')).toHaveTextContent('Pan & Tilt');
    const directions = [
      'up',
      'down',
      'left',
      'right',
      'up-left',
      'up-right',
      'down-left',
      'down-right',
    ];
    for (const dir of directions) {
      expect(screen.getByTestId(`liveview-ptz-${dir}-button`)).toBeInTheDocument();
    }
    expect(screen.getByTestId('liveview-ptz-home-button')).toBeInTheDocument();
  });

  it('should not show the LIVE indicator before playback starts', async () => {
    renderWithProviders(<LiveViewPage />);
    await screen.findByTestId('liveview-video');
    expect(screen.queryByTestId('liveview-live-indicator')).not.toBeInTheDocument();
  });

  it('renders the video element instead of the old placeholder', async () => {
    renderWithProviders(<LiveViewPage />);

    expect(await screen.findByTestId('liveview-video')).toBeInTheDocument();
    expect(screen.queryByTestId('liveview-stream-preview-title')).not.toBeInTheDocument();
  });

  it('shows a retry affordance when playback fails', async () => {
    renderWithProviders(<LiveViewPage />);
    await screen.findByTestId('liveview-video');

    const errorHandler = await waitForPlayerHandler('error');
    errorHandler('NetworkError', 'connection refused');

    expect(await screen.findByTestId('liveview-retry-button')).toBeInTheDocument();
  });

  it('fills the stream info card from decoded media info', async () => {
    renderWithProviders(<LiveViewPage />);
    await screen.findByTestId('liveview-video');

    const mediaHandler = await waitForPlayerHandler('media_info');
    mediaHandler({ width: 1280, height: 720, fps: 25, videoCodec: 'avc1.4d001f' });

    expect(await screen.findByTestId('liveview-resolution-value')).toHaveTextContent('1280x720');
    expect(screen.getByTestId('liveview-codec-value')).toHaveTextContent('H.264 Main@L3.1');
  });

  it('shows the measured frame rate from decoded frame deltas', async () => {
    renderWithProviders(<LiveViewPage />);
    await screen.findByTestId('liveview-video');

    const statsHandler = await waitForPlayerHandler('statistics_info');
    const now = vi.spyOn(performance, 'now');

    now.mockReturnValue(0);
    statsHandler({ speed: 128, decodedFrames: 0, droppedFrames: 0 });
    now.mockReturnValue(1000);
    statsHandler({ speed: 128, decodedFrames: 15, droppedFrames: 0 });

    await waitFor(() =>
      expect(screen.getByTestId('liveview-framerate-value')).toHaveTextContent('15 fps'),
    );
    now.mockRestore();
  });

  it('shows a placeholder rather than a fake number before stats arrive', async () => {
    renderWithProviders(<LiveViewPage />);
    await screen.findByTestId('liveview-video');

    expect(screen.getByTestId('liveview-resolution-value')).toHaveTextContent('—');
  });

  it('shows the stream URL actually in use and copies it', async () => {
    renderWithProviders(<LiveViewPage />);
    await screen.findByTestId('liveview-video');

    expect(screen.getByTestId('liveview-stream-url-value')).toHaveTextContent('/live/main.flv');

    fireEvent.click(screen.getByTestId('liveview-copy-url-button'));
    expect(mockWriteText).toHaveBeenCalledWith(expect.stringContaining('/live/main.flv'));
  });

  it('copies the stream URL over plain HTTP when clipboard is unavailable', async () => {
    const { toast } = await import('sonner');
    Object.defineProperty(navigator, 'clipboard', {
      value: undefined,
      writable: true,
      configurable: true,
    });
    Object.defineProperty(document, 'execCommand', {
      value: vi.fn().mockReturnValue(true),
      writable: true,
      configurable: true,
    });

    renderWithProviders(<LiveViewPage />);
    await screen.findByTestId('liveview-video');

    fireEvent.click(screen.getByTestId('liveview-copy-url-button'));

    await waitFor(() => {
      expect(toast.success).toHaveBeenCalledWith('Stream URL copied');
    });
  });

  it('updates the stream URL when switching to the sub stream', async () => {
    const user = userEvent.setup();
    renderWithProviders(<LiveViewPage />);
    await screen.findByTestId('liveview-video');

    await user.click(screen.getByTestId('liveview-sub-stream-button'));

    expect(screen.getByTestId('liveview-stream-url-value')).toHaveTextContent('/live/sub.flv');
  });

  it('replaces unmeasurable packet loss and latency with dropped frames', async () => {
    renderWithProviders(<LiveViewPage />);
    await screen.findByTestId('liveview-video');

    expect(screen.getByTestId('liveview-dropped-frames-value')).toBeInTheDocument();
    expect(screen.queryByTestId('liveview-packet-loss-value')).not.toBeInTheDocument();
    expect(screen.queryByTestId('liveview-latency-value')).not.toBeInTheDocument();
  });

  it('hides the LIVE indicator until playback actually starts', async () => {
    renderWithProviders(<LiveViewPage />);
    await screen.findByTestId('liveview-video');

    expect(screen.queryByTestId('liveview-live-indicator')).not.toBeInTheDocument();
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
        expect(screen.getByTestId('liveview-preset-preset1-label')).toHaveTextContent('Front Door');
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
        expect(screen.getByTestId('liveview-preset-preset1-label')).toHaveTextContent('Front Door');
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
        expect(screen.getByTestId('liveview-preset-preset1-label')).toHaveTextContent('Front Door');
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
        expect(screen.getByTestId('liveview-preset-preset1-label')).toHaveTextContent('Front Door');
      });

      const directions = [
        'up',
        'down',
        'left',
        'right',
        'up-left',
        'up-right',
        'down-left',
        'down-right',
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
        expect(screen.getByTestId('liveview-preset-preset1-button')).toBeInTheDocument();
      });

      const presetButton = screen.getByTestId('liveview-preset-preset1-button');
      await user.click(presetButton);

      await waitFor(() => {
        expect(mockGotoPreset).toHaveBeenCalledWith('ProfileToken1', 'preset1');
      });
    });

    it('should not save a machine-named preset straight from the add button', async () => {
      const user = userEvent.setup();
      const { setPreset: mockSetPreset } = await import('@/services/ptzService');
      renderWithProviders(<LiveViewPage />);

      await waitFor(() => {
        expect(screen.getByTestId('liveview-preset-preset1-label')).toHaveTextContent('Front Door');
      });

      await user.click(screen.getByTestId('liveview-add-preset-button'));

      // Naming was "Preset ${presets.length + 1}", which collides after a delete.
      expect(mockSetPreset).not.toHaveBeenCalled();
    });

    it('should not remove a preset on a bare trash click', async () => {
      const user = userEvent.setup();
      const { removePreset: mockRemovePreset } = await import('@/services/ptzService');
      renderWithProviders(<LiveViewPage />);

      await waitFor(() => {
        expect(screen.getByTestId('liveview-preset-preset1-delete-button')).toBeInTheDocument();
      });

      await user.click(screen.getByTestId('liveview-preset-preset1-delete-button'));

      await testDialogOpen('liveview-delete-preset-dialog');
      expect(mockRemovePreset).not.toHaveBeenCalled();
    });

    it('should remove the preset only after the delete is confirmed', async () => {
      const user = userEvent.setup();
      const { removePreset: mockRemovePreset } = await import('@/services/ptzService');
      renderWithProviders(<LiveViewPage />);

      await waitFor(() => {
        expect(screen.getByTestId('liveview-preset-preset2-delete-button')).toBeInTheDocument();
      });

      await user.click(screen.getByTestId('liveview-preset-preset2-delete-button'));
      await testDialogOpen('liveview-delete-preset-dialog');
      await user.click(screen.getByTestId('liveview-delete-preset-confirm'));

      await waitFor(() => {
        expect(mockRemovePreset).toHaveBeenCalledWith('ProfileToken1', 'preset2');
      });
    });

    it('should never remove a preset when the delete is cancelled', async () => {
      const user = userEvent.setup();
      const { removePreset: mockRemovePreset } = await import('@/services/ptzService');
      renderWithProviders(<LiveViewPage />);

      await waitFor(() => {
        expect(screen.getByTestId('liveview-preset-preset1-delete-button')).toBeInTheDocument();
      });

      await user.click(screen.getByTestId('liveview-preset-preset1-delete-button'));
      await testDialogOpen('liveview-delete-preset-dialog');
      await user.click(screen.getByTestId('liveview-delete-preset-cancel'));

      await waitFor(() => {
        testDialogClosed('liveview-delete-preset-dialog');
      });
      expect(mockRemovePreset).not.toHaveBeenCalled();
    });

    it('should re-enable the row after a failed delete', async () => {
      const user = userEvent.setup();
      const { removePreset: mockRemovePreset } = await import('@/services/ptzService');
      vi.mocked(mockRemovePreset).mockRejectedValueOnce(new Error('Device busy'));
      renderWithProviders(<LiveViewPage />);

      await waitFor(() => {
        expect(screen.getByTestId('liveview-preset-preset1-delete-button')).toBeInTheDocument();
      });

      await user.click(screen.getByTestId('liveview-preset-preset1-delete-button'));
      await testDialogOpen('liveview-delete-preset-dialog');
      await user.click(screen.getByTestId('liveview-delete-preset-confirm'));

      await waitFor(() => {
        expect(screen.getByTestId('liveview-preset-preset1-button')).toBeEnabled();
      });
    });

    it('falls back to the token for a preset the device never named', async () => {
      const { getPresets } = await import('@/services/ptzService');
      vi.mocked(getPresets).mockResolvedValueOnce([{ token: 'preset7', name: '' }]);
      renderWithProviders(<LiveViewPage />);

      expect(await screen.findByTestId('liveview-preset-preset7-label')).toHaveTextContent(
        'preset7',
      );
      expect(screen.getByLabelText('Delete preset preset7')).toBeInTheDocument();
    });

    it('labels the per-preset icon buttons for screen readers', async () => {
      renderWithProviders(<LiveViewPage />);

      expect(await screen.findByLabelText('Edit preset Front Door')).toBeInTheDocument();
      expect(screen.getByLabelText('Delete preset Front Door')).toBeInTheDocument();
    });

    it('should display preset names from backend', async () => {
      renderWithProviders(<LiveViewPage />);

      await waitFor(() => {
        expect(screen.getByTestId('liveview-preset-preset1-label')).toHaveTextContent('Front Door');
      });
      expect(screen.getByTestId('liveview-preset-preset2-label')).toHaveTextContent('Back Yard');
      expect(screen.getByTestId('liveview-preset-preset3-label')).toHaveTextContent('Garage');
    });

    it('should show error toast when PTZ move fails', async () => {
      const { continuousMove: mockContinuousMove } = await import('@/services/ptzService');
      const { toast } = await import('sonner');
      vi.mocked(mockContinuousMove).mockRejectedValueOnce(new Error('Connection refused'));

      renderWithProviders(<LiveViewPage />);

      await waitFor(() => {
        expect(screen.getByTestId('liveview-preset-preset1-label')).toHaveTextContent('Front Door');
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
});
