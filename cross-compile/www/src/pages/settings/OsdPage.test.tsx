/**
 * OsdPage Tests
 */
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { getOsdSettings, setOsd, setOsdEnabled } from '@/services/osdService';
import { renderWithProviders, waitForPageLoad } from '@/test/componentTestHelpers';

import OsdPage from './OsdPage';

vi.mock('@/services/osdService', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/services/osdService')>();
  return {
    ...actual,
    getOsdSettings: vi.fn(),
    setOsd: vi.fn(),
    setOsdEnabled: vi.fn(),
  };
});

const mockSettings = {
  name: {
    token: 'osd_name' as const,
    enabled: true,
    position: 'UpperLeft' as const,
    text: 'CAM1',
    videoSourceToken: 'VS0',
  },
  datetime: {
    token: 'osd_datetime' as const,
    enabled: true,
    position: 'LowerRight' as const,
    dateFormat: 'yyyy-MM-dd' as const,
    timeFormat: 'HH:mm:ss' as const,
    videoSourceToken: 'VS0',
  },
  appearance: { color: 1, alpha: 80 },
};

describe('OsdPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getOsdSettings).mockResolvedValue(mockSettings);
    vi.mocked(setOsd).mockResolvedValue(undefined);
    vi.mocked(setOsdEnabled).mockResolvedValue(undefined);
  });

  it('should render loading state', () => {
    vi.mocked(getOsdSettings).mockImplementation(() => new Promise(() => {}));
    renderWithProviders(<OsdPage />);
    expect(screen.getByTestId('osd-loading')).toBeInTheDocument();
  });

  it('should render error state when getOsdSettings rejects', async () => {
    vi.mocked(getOsdSettings).mockRejectedValue(new Error('failed'));
    renderWithProviders(<OsdPage />);
    expect(await screen.findByTestId('osd-error')).toBeInTheDocument();
  });

  it('should render fetched values', async () => {
    renderWithProviders(<OsdPage />);
    await waitForPageLoad('osd-title');
    expect(screen.getByTestId('osd-name-text-input')).toHaveValue('CAM1');
    expect(screen.getByTestId('osd-alpha-value')).toHaveTextContent('80%');
  });

  it('should save when corner changes via save button', async () => {
    const user = userEvent.setup();
    renderWithProviders(<OsdPage />);
    await waitForPageLoad('osd-title');

    await user.click(screen.getByTestId('osd-name-position-select'));
    await user.click(screen.getByTestId('osd-name-pos-LowerLeft'));
    await user.click(screen.getByTestId('osd-save-button'));

    await waitFor(() => {
      expect(setOsd).toHaveBeenCalled();
    });
    expect(vi.mocked(setOsd).mock.calls.some((c) => c[0].position === 'LowerLeft')).toBe(true);
  });

  it('should show ASCII validation and not mutate on non-ASCII input', async () => {
    const user = userEvent.setup();
    renderWithProviders(<OsdPage />);
    await waitForPageLoad('osd-title');

    const input = screen.getByTestId('osd-name-text-input');
    await user.clear(input);
    await user.type(input, 'Ogród');

    expect(screen.getByTestId('osd-name-ascii-error')).toBeInTheDocument();
    expect(screen.getByTestId('osd-save-button')).toBeDisabled();

    await user.click(screen.getByTestId('osd-save-button'));
    expect(setOsd).not.toHaveBeenCalled();
  });

  it('should disable the timestamp without pushing text for it', async () => {
    const user = userEvent.setup();
    renderWithProviders(<OsdPage />);
    await waitForPageLoad('osd-title');

    await user.click(screen.getByTestId('osd-datetime-enabled-switch'));
    await user.click(screen.getByTestId('osd-save-button'));

    await waitFor(() => {
      expect(setOsdEnabled).toHaveBeenCalled();
    });
    // DeleteOSD for the timestamp...
    expect(
      vi.mocked(setOsdEnabled).mock.calls.some((c) => c[0].token === 'osd_datetime' && !c[1]),
    ).toBe(true);
    // ...and no SetOSD for it, since a disabled OSD does not exist in ONVIF.
    expect(vi.mocked(setOsd).mock.calls.some((c) => c[0].token === 'osd_datetime')).toBe(false);
  });
});
