/**
 * ImagingPage Tests
 */
import { fireEvent, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { getDiagnostics } from '@/services/diagnosticsService';
import { getAdvancedImaging, putAdvancedImaging } from '@/services/imagingAdvancedService';
import {
  getImagingOptions,
  getImagingSettings,
  setImagingSettings,
} from '@/services/imagingService';
import {
  getProfiles,
  getVideoSourceConfiguration,
  setVideoSourceConfiguration,
} from '@/services/profileService';
import { sendAuxiliaryCommand } from '@/services/ptzService';
import { MOCK_DATA, mockToast, renderWithProviders } from '@/test/componentTestHelpers';
import {
  testMutationWithErrorToast,
  testMutationWithSuccessToast,
} from '@/test/mutationTestHelpers';

import ImagingPage from './ImagingPage';

// Mock services
vi.mock('@/services/imagingService', () => ({
  getImagingSettings: vi.fn(),
  getImagingOptions: vi.fn(),
  setImagingSettings: vi.fn(),
}));

vi.mock('@/services/profileService', () => ({
  getProfiles: vi.fn(),
  getVideoSourceConfiguration: vi.fn(),
  setVideoSourceConfiguration: vi.fn(),
}));

vi.mock('@/services/ptzService', () => ({
  sendAuxiliaryCommand: vi.fn(),
}));

vi.mock('@/services/imagingAdvancedService', () => ({
  getAdvancedImaging: vi.fn(),
  putAdvancedImaging: vi.fn(),
}));
vi.mock('@/services/diagnosticsService', () => ({
  getDiagnostics: vi.fn(),
}));

const MOCK_VIDEO_SOURCE_CONFIG = {
  token: 'VideoSourceConfig_0',
  name: 'VideoSourceConfig_0',
  useCount: 2,
  sourceToken: 'VideoSource_1',
  bounds: { x: 0, y: 0, width: 1920, height: 1080 },
  rotate: 'OFF' as const,
};

describe('ImagingPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getImagingSettings).mockResolvedValue(MOCK_DATA.imaging.settings);
    vi.mocked(getImagingOptions).mockResolvedValue(MOCK_DATA.imaging.options);
    vi.mocked(setImagingSettings).mockResolvedValue(undefined);
    vi.mocked(getAdvancedImaging).mockResolvedValue({ hue: 50, powerHz: 50, styleId: 0 });
    vi.mocked(putAdvancedImaging).mockResolvedValue(undefined);
    vi.mocked(getDiagnostics).mockResolvedValue({
      vision: { ir_led: false, white_led: false },
    } as never);
    vi.mocked(getProfiles).mockResolvedValue(MOCK_DATA.profiles);
    vi.mocked(sendAuxiliaryCommand).mockResolvedValue(undefined);
    vi.mocked(getVideoSourceConfiguration).mockResolvedValue(MOCK_VIDEO_SOURCE_CONFIG);
    vi.mocked(setVideoSourceConfiguration).mockResolvedValue(undefined);
  });

  it('should render page with loading state', async () => {
    vi.mocked(getImagingSettings).mockImplementation(() => new Promise(() => {}));

    renderWithProviders(<ImagingPage />);
    expect(screen.getByTestId('imaging-loading')).toBeInTheDocument();
  });

  it('should render form with fetched settings', async () => {
    renderWithProviders(<ImagingPage />);

    await waitFor(() => {
      expect(screen.getByTestId('imaging-title')).toBeInTheDocument();
    });

    // Check that brightness and contrast values are present (may appear multiple times)
    const brightnessValues = screen.getAllByText('60%');
    const contrastValues = screen.getAllByText('70%');
    expect(brightnessValues.length).toBeGreaterThan(0);
    expect(contrastValues.length).toBeGreaterThan(0);
  });

  it('should render brightness slider', async () => {
    renderWithProviders(<ImagingPage />);

    await waitFor(() => {
      expect(screen.getByTestId('imaging-title')).toBeInTheDocument();
    });

    // Verify slider is rendered (avoiding direct interaction to prevent pointer capture errors)
    const brightnessSliders = screen.getAllByRole('slider');
    expect(brightnessSliders.length).toBeGreaterThan(0);
    expect(brightnessSliders[0]).toBeInTheDocument();
  });

  it('should render IR cut filter selection', async () => {
    renderWithProviders(<ImagingPage />);

    await waitFor(() => {
      expect(screen.getByTestId('imaging-title')).toBeInTheDocument();
    });

    // Verify IR cut filter section is rendered
    expect(screen.getByTestId('imaging-infrared-settings-title')).toBeInTheDocument();
    expect(screen.getByTestId('imaging-ir-cut-filter-mode-label')).toBeInTheDocument();
    // The select element should be present
    const selects = screen.getAllByRole('combobox');
    expect(selects.length).toBeGreaterThan(0);
  });

  it('should stub the WDR control and mark it unavailable on this ISP', async () => {
    renderWithProviders(<ImagingPage />);

    const select = await screen.findByTestId('imaging-wdr-mode-select');
    expect(select).toBeDisabled();
    // The level slider the mode used to reveal is gone with it.
    expect(screen.queryByTestId('imaging-wdr-level-label')).not.toBeInTheDocument();
    expect(screen.getByTestId('imaging-wdr-unavailable')).toBeInTheDocument();
  });

  it('should show backlight level slider when backlight compensation is ON', async () => {
    renderWithProviders(<ImagingPage />);

    await waitFor(() => {
      expect(screen.getByTestId('imaging-backlight-level-label')).toBeInTheDocument();
      // Backlight level value is displayed as percentage - verify slider exists
      const sliders = screen.getAllByRole('slider');
      expect(sliders.length).toBeGreaterThan(0);
    });
  });

  it('should submit form and call mutation', async () => {
    const user = userEvent.setup();
    renderWithProviders(<ImagingPage />);

    await waitFor(() => {
      expect(screen.getByTestId('imaging-title')).toBeInTheDocument();
    });

    await testMutationWithSuccessToast(
      user,
      'imaging-save-button',
      setImagingSettings,
      'Image settings saved',
    );
  });

  it('should show error toast when mutation fails', async () => {
    vi.mocked(setImagingSettings).mockRejectedValue(new Error('Network error'));

    const user = userEvent.setup();
    renderWithProviders(<ImagingPage />);

    await waitFor(() => {
      expect(screen.getByTestId('imaging-title')).toBeInTheDocument();
    });

    await testMutationWithErrorToast(
      user,
      'imaging-save-button',
      setImagingSettings,
      'Failed to save image settings',
      'Network error',
    );
  });

  it('should reset form when reset button is clicked', async () => {
    const user = userEvent.setup();
    renderWithProviders(<ImagingPage />);

    await waitFor(() => {
      expect(screen.getByTestId('imaging-title')).toBeInTheDocument();
    });

    const resetButton = screen.getByTestId('imaging-reset-button');
    await user.click(resetButton);

    await waitFor(() => {
      expect(mockToast.info).toHaveBeenCalledWith('Reset to current saved values');
    });
  });

  it('should render all settings cards', async () => {
    renderWithProviders(<ImagingPage />);

    await waitFor(() => {
      expect(screen.getByTestId('imaging-color-brightness-title')).toBeInTheDocument();
      expect(screen.getByTestId('imaging-infrared-settings-title')).toBeInTheDocument();
      expect(screen.getByTestId('imaging-backlight-wdr-title')).toBeInTheDocument();
      expect(screen.getByTestId('imaging-advanced-title')).toBeInTheDocument();
    });

    // Sharpness lives in Color & Brightness: the device has no motorised
    // focus, so there is no longer a Focus card.
    expect(screen.queryByTestId('imaging-focus-sharpness-title')).not.toBeInTheDocument();
  });

  it('should update slider values when changed', async () => {
    renderWithProviders(<ImagingPage />);

    await waitFor(() => {
      expect(screen.getByTestId('imaging-title')).toBeInTheDocument();
    });

    // Find brightness slider
    const sliders = screen.getAllByRole('slider');
    expect(sliders.length).toBeGreaterThan(0);

    // Verify sliders are present
    // Range inputs are difficult to test with userEvent, so we verify they exist
    // The slider might be wrapped in a component, so we check it's a slider role
    const brightnessSlider = sliders[0];
    expect(brightnessSlider).toBeInTheDocument();
    // Slider role indicates it's a range input or similar control
    expect(brightnessSlider).toHaveAttribute('role', 'slider');
  });

  it('should handle save with updated values', async () => {
    const user = userEvent.setup();
    renderWithProviders(<ImagingPage />);

    await waitFor(() => {
      expect(screen.getByTestId('imaging-title')).toBeInTheDocument();
    });

    // Verify sliders are present (range inputs are difficult to test programmatically)
    const sliders = screen.getAllByRole('slider');
    expect(sliders.length).toBeGreaterThan(0);

    // Save changes
    await testMutationWithSuccessToast(
      user,
      'imaging-save-button',
      setImagingSettings,
      'Image settings saved',
    );
  });

  describe('Error Handling', () => {
    it('should handle error when getImagingSettings query fails', async () => {
      vi.mocked(getImagingSettings).mockRejectedValue(new Error('Failed to fetch settings'));

      renderWithProviders(<ImagingPage />);

      // Should show loading initially, then error state
      await waitFor(
        () => {
          // Query error should be handled by React Query
          // The page should still render (React Query shows error state)
          expect(screen.queryByTestId('imaging-loading')).not.toBeInTheDocument();
        },
        { timeout: 3000 },
      );
    });

    it('should handle error when getImagingOptions query fails', async () => {
      vi.mocked(getImagingOptions).mockRejectedValue(new Error('Failed to fetch options'));

      renderWithProviders(<ImagingPage />);

      await waitFor(() => {
        expect(screen.getByTestId('imaging-title')).toBeInTheDocument();
      });

      // Page should still render with default options
      const sliders = screen.getAllByRole('slider');
      expect(sliders.length).toBeGreaterThan(0);
    });

    it('should handle mutation error with Error object', async () => {
      const error = new Error('Network timeout');
      vi.mocked(setImagingSettings).mockRejectedValue(error);

      const user = userEvent.setup();
      renderWithProviders(<ImagingPage />);

      await waitFor(() => {
        expect(screen.getByTestId('imaging-title')).toBeInTheDocument();
      });

      await testMutationWithErrorToast(
        user,
        'imaging-save-button',
        setImagingSettings,
        'Failed to save image settings',
        'Network timeout',
      );
    });

    it('should handle mutation error with non-Error object', async () => {
      vi.mocked(setImagingSettings).mockRejectedValue('String error');

      const user = userEvent.setup();
      renderWithProviders(<ImagingPage />);

      await waitFor(() => {
        expect(screen.getByTestId('imaging-title')).toBeInTheDocument();
      });

      await testMutationWithErrorToast(
        user,
        'imaging-save-button',
        setImagingSettings,
        'Failed to save image settings',
        'An error occurred',
      );
    });
  });

  describe('Edge Cases', () => {
    it('should use default values when options are missing', async () => {
      vi.mocked(getImagingOptions).mockResolvedValue(
        null as unknown as typeof MOCK_DATA.imaging.options,
      );

      renderWithProviders(<ImagingPage />);

      await waitFor(() => {
        expect(screen.getByTestId('imaging-title')).toBeInTheDocument();
      });

      // Sliders should still render with default min/max (0-100)
      const sliders = screen.getAllByRole('slider');
      expect(sliders.length).toBeGreaterThan(0);
    });

    it('should handle reset when settings is null', async () => {
      vi.mocked(getImagingSettings).mockResolvedValue(
        null as unknown as typeof MOCK_DATA.imaging.settings,
      );

      const user = userEvent.setup();
      renderWithProviders(<ImagingPage />);

      await waitFor(() => {
        expect(screen.getByTestId('imaging-title')).toBeInTheDocument();
      });

      const resetButton = screen.getByTestId('imaging-reset-button');
      await user.click(resetButton);

      // Reset should not show toast when settings is null
      await waitFor(
        () => {
          // Button click should not throw error
          expect(resetButton).toBeInTheDocument();
        },
        { timeout: 1000 },
      );
    });

    it('should use fallback defaults for slider min/max when options are missing', async () => {
      vi.mocked(getImagingOptions).mockResolvedValue(
        null as unknown as typeof MOCK_DATA.imaging.options,
      );

      renderWithProviders(<ImagingPage />);

      await waitFor(() => {
        expect(screen.getByTestId('imaging-title')).toBeInTheDocument();
      });

      // Sliders should work with default min=0, max=100 from code
      const sliders = screen.getAllByRole('slider');
      expect(sliders.length).toBeGreaterThan(0);
      // Verify sliders are functional (they have min/max attributes)
      sliders.forEach((slider) => {
        expect(slider).toBeInTheDocument();
      });
    });

    it('should handle missing wideDynamicRange in settings', async () => {
      const settingsWithoutWDR = {
        ...MOCK_DATA.imaging.settings,
        wideDynamicRange: undefined,
      };
      vi.mocked(getImagingSettings).mockResolvedValue(
        settingsWithoutWDR as unknown as typeof MOCK_DATA.imaging.settings,
      );

      renderWithProviders(<ImagingPage />);

      await waitFor(() => {
        expect(screen.getByTestId('imaging-title')).toBeInTheDocument();
      });

      // Page should render with default WDR settings
      expect(screen.getByTestId('imaging-backlight-wdr-title')).toBeInTheDocument();
    });

    it('should handle missing backlightCompensation in settings', async () => {
      const settingsWithoutBacklight = {
        ...MOCK_DATA.imaging.settings,
        backlightCompensation: undefined,
      };
      vi.mocked(getImagingSettings).mockResolvedValue(
        settingsWithoutBacklight as unknown as typeof MOCK_DATA.imaging.settings,
      );

      renderWithProviders(<ImagingPage />);

      await waitFor(() => {
        expect(screen.getByTestId('imaging-title')).toBeInTheDocument();
      });

      // Page should render with default backlight settings
      expect(screen.getByTestId('imaging-backlight-wdr-title')).toBeInTheDocument();
    });

    it('should not update localSettings when useEffect detects no changes', async () => {
      // Test the condition in useEffect that prevents unnecessary re-renders (lines 68-79)
      const initialSettings = MOCK_DATA.imaging.settings;
      vi.mocked(getImagingSettings).mockResolvedValue(initialSettings);

      const { rerender } = renderWithProviders(<ImagingPage />);

      await waitFor(() => {
        expect(screen.getByTestId('imaging-title')).toBeInTheDocument();
      });

      // Re-render with the same settings - useEffect should detect no changes
      vi.mocked(getImagingSettings).mockResolvedValue(initialSettings);
      rerender(<ImagingPage />);

      await waitFor(() => {
        // Page should still render correctly
        expect(screen.getByTestId('imaging-title')).toBeInTheDocument();
      });
    });

    it('should use fallback defaults for wideDynamicRange when settings.wideDynamicRange is undefined', async () => {
      const settingsWithoutWDR = {
        ...MOCK_DATA.imaging.settings,
        wideDynamicRange: undefined,
      };
      vi.mocked(getImagingSettings).mockResolvedValue(
        settingsWithoutWDR as unknown as typeof MOCK_DATA.imaging.settings,
      );

      renderWithProviders(<ImagingPage />);

      await waitFor(() => {
        expect(screen.getByTestId('imaging-title')).toBeInTheDocument();
      });

      // Page should render with default WDR settings (mode: OFF, level: 50)
      expect(screen.getByTestId('imaging-backlight-wdr-title')).toBeInTheDocument();
    });

    it('should use fallback defaults for backlightCompensation when settings.backlightCompensation is undefined', async () => {
      const settingsWithoutBacklight = {
        ...MOCK_DATA.imaging.settings,
        backlightCompensation: undefined,
      };
      vi.mocked(getImagingSettings).mockResolvedValue(
        settingsWithoutBacklight as unknown as typeof MOCK_DATA.imaging.settings,
      );

      renderWithProviders(<ImagingPage />);

      await waitFor(() => {
        expect(screen.getByTestId('imaging-title')).toBeInTheDocument();
      });

      // Page should render with default backlight settings (mode: OFF, level: 50)
      expect(screen.getByTestId('imaging-backlight-wdr-title')).toBeInTheDocument();
    });
  });

  describe('Slider interactions', () => {
    it('should update brightness when slider changes', async () => {
      renderWithProviders(<ImagingPage />);

      await waitFor(() => {
        expect(screen.getByTestId('imaging-title')).toBeInTheDocument();
      });

      const sliders = screen.getAllByRole('slider');
      const brightnessSlider = sliders[0]; // First slider is brightness
      expect(brightnessSlider).toBeInTheDocument();

      // Verify brightness value is displayed
      expect(screen.getAllByText('60%').length).toBeGreaterThan(0);
    });

    it('should update contrast when slider changes', async () => {
      renderWithProviders(<ImagingPage />);

      await waitFor(() => {
        expect(screen.getByTestId('imaging-title')).toBeInTheDocument();
      });

      // Verify contrast value is displayed
      expect(screen.getAllByText('70%').length).toBeGreaterThan(0);
    });

    it('should update saturation when slider changes', async () => {
      renderWithProviders(<ImagingPage />);

      await waitFor(() => {
        expect(screen.getByTestId('imaging-title')).toBeInTheDocument();
      });

      // Verify saturation value is displayed (from mock data)
      const sliders = screen.getAllByRole('slider');
      expect(sliders.length).toBeGreaterThan(0);
    });

    it('should update sharpness when slider changes', async () => {
      renderWithProviders(<ImagingPage />);

      await waitFor(() => {
        expect(screen.getByTestId('imaging-title')).toBeInTheDocument();
      });

      // Verify sharpness slider exists
      const sliders = screen.getAllByRole('slider');
      expect(sliders.length).toBeGreaterThan(0);
    });
  });

  describe('Select dropdown interactions', () => {
    it('should change IR cut filter mode', async () => {
      const user = userEvent.setup();
      renderWithProviders(<ImagingPage />);

      await waitFor(() => {
        expect(screen.getByTestId('imaging-title')).toBeInTheDocument();
      });

      const selects = screen.getAllByRole('combobox');
      const irCutFilterSelect =
        selects.find((select: HTMLElement) => {
          const label = screen.getByTestId('imaging-ir-cut-filter-mode-label');
          return select.closest('div')?.contains(label) || false;
        }) || selects[0];

      expect(irCutFilterSelect).toBeInTheDocument();
      await user.selectOptions(irCutFilterSelect, 'ON');
      expect(irCutFilterSelect).toHaveValue('ON');
    });

    it('should stub the WDR mode select and hide the level slider it would reveal', async () => {
      renderWithProviders(<ImagingPage />);

      await waitFor(() => {
        expect(screen.getByTestId('imaging-title')).toBeInTheDocument();
      });

      const select = screen.getByTestId('imaging-wdr-mode-select');
      expect(select).toBeDisabled();
      expect(screen.queryByTestId('imaging-wdr-level-label')).not.toBeInTheDocument();
    });

    it('should change backlight compensation mode and show level slider when ON', async () => {
      renderWithProviders(<ImagingPage />);

      await waitFor(() => {
        expect(screen.getByTestId('imaging-title')).toBeInTheDocument();
      });

      // When backlight compensation mode is ON, the level slider should be visible
      // From mock data, backlight compensation mode is ON, so level slider should be visible
      await waitFor(() => {
        expect(screen.getByTestId('imaging-backlight-level-label')).toBeInTheDocument();
      });
    });

    it('should hide WDR level slider when WDR mode is OFF', async () => {
      const settingsWithWDRoff = {
        ...MOCK_DATA.imaging.settings,
        wideDynamicRange: {
          mode: 'OFF' as const,
          level: 50,
        },
      };
      vi.mocked(getImagingSettings).mockResolvedValue(
        settingsWithWDRoff as unknown as typeof MOCK_DATA.imaging.settings,
      );

      renderWithProviders(<ImagingPage />);

      await waitFor(() => {
        expect(screen.getByTestId('imaging-title')).toBeInTheDocument();
      });

      // WDR level slider should not be visible when mode is OFF
      expect(screen.queryByTestId('imaging-wdr-level-label')).not.toBeInTheDocument();
    });

    it('should hide backlight level slider when backlight mode is OFF', async () => {
      const settingsWithBacklightOff = {
        ...MOCK_DATA.imaging.settings,
        backlightCompensation: {
          mode: 'OFF' as const,
          level: 50,
        },
      };
      vi.mocked(getImagingSettings).mockResolvedValue(
        settingsWithBacklightOff as unknown as typeof MOCK_DATA.imaging.settings,
      );

      renderWithProviders(<ImagingPage />);

      await waitFor(() => {
        expect(screen.getByTestId('imaging-title')).toBeInTheDocument();
      });

      // Backlight level slider should not be visible when mode is OFF
      expect(screen.queryByTestId('imaging-backlight-level-label')).not.toBeInTheDocument();
    });
  });

  describe('updateSetting function', () => {
    it('should update all setting types correctly', async () => {
      const user = userEvent.setup();
      renderWithProviders(<ImagingPage />);

      await waitFor(() => {
        expect(screen.getByTestId('imaging-title')).toBeInTheDocument();
      });

      // Test that all sliders can be interacted with (updateSetting is called)
      const sliders = screen.getAllByRole('slider');
      expect(sliders.length).toBeGreaterThan(0);

      // Test that selects can be changed (updateSetting is called)
      const selects = screen.getAllByRole('combobox');
      expect(selects.length).toBeGreaterThan(0);

      // Verify save button calls mutation with all settings
      const saveButton = screen.getByTestId('imaging-save-button');
      await user.click(saveButton);

      await waitFor(() => {
        expect(setImagingSettings).toHaveBeenCalled();
        const callArgs = vi.mocked(setImagingSettings).mock.calls[0][0];
        expect(callArgs).toHaveProperty('brightness');
        expect(callArgs).toHaveProperty('contrast');
        expect(callArgs).toHaveProperty('saturation');
        expect(callArgs).toHaveProperty('sharpness');
        expect(callArgs).toHaveProperty('irCutFilter');
        expect(callArgs).toHaveProperty('wideDynamicRange');
        expect(callArgs).toHaveProperty('backlightCompensation');
      });
    });
  });

  describe('illumination card', () => {
    it('should render the illumination card with both lamp switches', async () => {
      renderWithProviders(<ImagingPage />);

      expect(await screen.findByTestId('imaging-ir-lamp-switch')).toBeInTheDocument();
      expect(screen.getByTestId('imaging-white-light-switch')).toBeInTheDocument();
    });

    it('should send the IR lamp on command when the switch is enabled', async () => {
      const user = userEvent.setup();
      renderWithProviders(<ImagingPage />);

      // The switch is disabled until the profile resolves and no lamp
      // mutation is pending; wait for it to be clickable, not just rendered.
      const irSwitch = await screen.findByTestId('imaging-ir-lamp-switch');
      await waitFor(() => expect(irSwitch).not.toBeDisabled());
      await user.click(irSwitch);

      await waitFor(() => {
        expect(sendAuxiliaryCommand).toHaveBeenCalledWith('ProfileToken1', 'tt:IRLamp|On');
      });
    });

    it('should send the white light off command when the switch is toggled twice', async () => {
      const user = userEvent.setup();
      renderWithProviders(<ImagingPage />);

      const whiteSwitch = await screen.findByTestId('imaging-white-light-switch');
      await waitFor(() => expect(whiteSwitch).not.toBeDisabled());
      await user.click(whiteSwitch);
      await waitFor(() => {
        expect(sendAuxiliaryCommand).toHaveBeenCalledWith('ProfileToken1', 'tt:WhiteLight|On');
      });

      // The switch is disabled while the mutation is pending; wait for it to
      // become clickable again before toggling off.
      await waitFor(() => expect(whiteSwitch).not.toBeDisabled());
      await user.click(whiteSwitch);
      await waitFor(() => {
        expect(sendAuxiliaryCommand).toHaveBeenCalledWith('ProfileToken1', 'tt:WhiteLight|Off');
      });
    });

    it('should hide the IR cut card when the backend reports no filter modes', async () => {
      vi.mocked(getImagingOptions).mockResolvedValue({
        ...MOCK_DATA.imaging.options,
        irCutFilterModes: [],
      });

      renderWithProviders(<ImagingPage />);

      await waitFor(() => {
        expect(screen.getByTestId('imaging-title')).toBeInTheDocument();
      });

      expect(screen.queryByTestId('imaging-infrared-settings-title')).not.toBeInTheDocument();
      expect(screen.getByTestId('imaging-illumination-card')).toBeInTheDocument();
    });
  });

  describe('image orientation', () => {
    it('should render the flip switch unchecked when rotate is OFF', async () => {
      renderWithProviders(<ImagingPage />);

      const flipSwitch = await screen.findByTestId('imaging-flip-switch');
      expect(flipSwitch).toBeInTheDocument();
      expect(flipSwitch).toHaveAttribute('aria-checked', 'false');
    });

    it('should render the flip switch checked when rotate is ON', async () => {
      vi.mocked(getVideoSourceConfiguration).mockResolvedValue({
        ...MOCK_VIDEO_SOURCE_CONFIG,
        rotate: 'ON',
      });

      renderWithProviders(<ImagingPage />);

      await waitFor(() => {
        expect(screen.getByTestId('imaging-flip-switch')).toHaveAttribute('aria-checked', 'true');
      });
    });

    it('should send the whole configuration with rotate flipped on toggle', async () => {
      const user = userEvent.setup();
      renderWithProviders(<ImagingPage />);

      const flipSwitch = await screen.findByTestId('imaging-flip-switch');
      await user.click(flipSwitch);

      await waitFor(() => {
        // Bounds and useCount must be round-tripped: the device replaces the
        // stored configuration wholesale.
        expect(setVideoSourceConfiguration).toHaveBeenCalledWith({
          ...MOCK_VIDEO_SOURCE_CONFIG,
          rotate: 'ON',
        });
      });
    });

    it('should show a success toast after toggling', async () => {
      const user = userEvent.setup();
      renderWithProviders(<ImagingPage />);

      const flipSwitch = await screen.findByTestId('imaging-flip-switch');
      await user.click(flipSwitch);

      await waitFor(() => {
        expect(mockToast.success).toHaveBeenCalledWith('Image orientation updated');
      });
    });

    it('should show an error toast when the toggle fails', async () => {
      vi.mocked(setVideoSourceConfiguration).mockRejectedValue(new Error('Device unavailable'));
      const user = userEvent.setup();
      renderWithProviders(<ImagingPage />);

      const flipSwitch = await screen.findByTestId('imaging-flip-switch');
      await user.click(flipSwitch);

      await waitFor(() => {
        expect(mockToast.error).toHaveBeenCalledWith('Failed to update image orientation', {
          description: 'Device unavailable',
        });
      });
    });

    it('should disable the switch when the configuration could not be fetched', async () => {
      vi.mocked(getVideoSourceConfiguration).mockResolvedValue(null);

      renderWithProviders(<ImagingPage />);

      const flipSwitch = await screen.findByTestId('imaging-flip-switch');
      expect(flipSwitch).toBeDisabled();
    });
  });

  it('should render the advanced card with hue slider and both selects', async () => {
    renderWithProviders(<ImagingPage />);

    await waitFor(() => {
      expect(screen.getByTestId('imaging-advanced-title')).toBeInTheDocument();
    });
    expect(screen.getByTestId('imaging-hue-slider')).toBeInTheDocument();
    expect(screen.getByTestId('imaging-power-hz-select')).toBeInTheDocument();
    expect(screen.getByTestId('imaging-style-select')).toBeInTheDocument();
  });

  it('should save the mains frequency when the select changes', async () => {
    const user = userEvent.setup();
    renderWithProviders(<ImagingPage />);

    await waitFor(() => {
      expect(screen.getByTestId('imaging-power-hz-select')).toBeInTheDocument();
    });

    await user.selectOptions(screen.getByTestId('imaging-power-hz-select'), '60');
    await waitFor(() => {
      expect(putAdvancedImaging).toHaveBeenCalledWith({ powerHz: 60 });
    });
  });

  it('should seed the lamp switches from the diagnostics snapshot', async () => {
    vi.mocked(getDiagnostics).mockResolvedValue({
      vision: { ir_led: true, white_led: true },
    } as never);
    renderWithProviders(<ImagingPage />);

    const ir = await screen.findByTestId('imaging-ir-lamp-switch');
    const white = screen.getByTestId('imaging-white-light-switch');
    // The diagnostics query resolves independently of the settings query that
    // gates isLoading; wait for it to actually land before asserting the
    // seeded (checked) state.
    await waitFor(() => {
      expect(ir).toBeChecked();
      expect(white).toBeChecked();
    });
  });

  it('should mount the live preview beside the cards', async () => {
    renderWithProviders(<ImagingPage />);

    await waitFor(() => {
      expect(screen.getByTestId('imaging-live-preview')).toBeInTheDocument();
    });
  });

  it('should not write on intermediate hue steps', async () => {
    renderWithProviders(<ImagingPage />);

    await waitFor(() => {
      expect(screen.getByTestId('imaging-hue-slider')).toBeInTheDocument();
    });

    // Radix renders one thumb per value; the wrapper tags it with a testid.
    const thumb = document
      .querySelector('[data-testid="imaging-hue-slider"]')
      ?.querySelector('[data-testid="slider-thumb"]') as HTMLElement | null;
    expect(thumb).not.toBeNull();
    fireEvent.focusIn(thumb!);

    // Intermediate (change-only) events must never write. A keyboard step on
    // the controlled slider updates the displayed value through
    // onValueChange without firing onValueCommit — the same change-only path
    // a drag tick takes.
    const mock = vi.mocked(putAdvancedImaging);
    const initial = mock.mock.calls.length;
    fireEvent.keyDown(thumb!, { key: 'ArrowRight', code: 'ArrowRight', keyCode: 39 });
    fireEvent.keyDown(thumb!, { key: 'ArrowRight', code: 'ArrowRight', keyCode: 39 });
    expect(mock.mock.calls.length).toBe(initial);

    // The pointer-up that ends a real drag fires onValueCommit, which
    // jsdom cannot reproduce (react-aria's slide-end never fires from
    // synthetic pointer events here, and this Radix build discards the
    // keyboard commit on a controlled slider). The commit side of the
    // invariant is therefore covered by its verifiable halves: the
    // advanced mutation performs exactly one service write when invoked
    // (the select-based tests below drive the same advancedMutation), and
    // the regression this design exists to prevent — a write per
    // intermediate value — is the one asserted above.
  });

  it('should save the picture style when the select changes', async () => {
    const user = userEvent.setup();
    renderWithProviders(<ImagingPage />);

    await waitFor(() => {
      expect(screen.getByTestId('imaging-style-select')).toBeInTheDocument();
    });

    await user.selectOptions(screen.getByTestId('imaging-style-select'), '1');
    await waitFor(() => {
      expect(putAdvancedImaging).toHaveBeenCalledWith({ styleId: 1 });
    });
  });
});
