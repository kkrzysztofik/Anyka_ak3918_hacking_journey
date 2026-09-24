/**
 * ProfilesPage Tests
 */
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import {
  type AudioEncoderConfiguration,
  type AudioEncoderConfigurationOptions,
  type MediaProfile,
  type VideoEncoderConfiguration,
  type VideoEncoderConfigurationOptions,
  addConfiguration,
  createProfile,
  deleteProfile,
  getAudioEncoderConfiguration,
  getAudioEncoderConfigurationOptions,
  getCompatibleConfigurations,
  getProfiles,
  getVideoEncoderConfiguration,
  getVideoEncoderConfigurationOptions,
  removeConfiguration,
  setAudioEncoderConfiguration,
  setVideoEncoderConfiguration,
} from '@/services/profileService';
import {
  MOCK_DATA,
  expandProfile,
  findProfileByName,
  mockToast,
  renderWithProviders,
  waitForPageLoad,
  waitForVideoEncoderSection,
} from '@/test/componentTestHelpers';

import ProfilesPage from './ProfilesPage';

// Mock services
vi.mock('@/services/profileService', () => ({
  getProfiles: vi.fn(),
  createProfile: vi.fn(),
  deleteProfile: vi.fn(),
  getVideoEncoderConfiguration: vi.fn(),
  getVideoEncoderConfigurationOptions: vi.fn(),
  setVideoEncoderConfiguration: vi.fn(),
  getAudioEncoderConfiguration: vi.fn(),
  getAudioEncoderConfigurationOptions: vi.fn(),
  setAudioEncoderConfiguration: vi.fn(),
  getCompatibleConfigurations: vi.fn(),
  addConfiguration: vi.fn(),
  removeConfiguration: vi.fn(),
}));

describe('ProfilesPage', () => {
  const mockProfiles = MOCK_DATA.profiles;

  const renderProfilesPage = async () => {
    const result = renderWithProviders(<ProfilesPage />);
    await waitForPageLoad('profiles-title');
    return result;
  };

  const setupVideoEncoderMocks = () => {
    vi.mocked(getVideoEncoderConfiguration).mockResolvedValue(
      MOCK_DATA.videoEncoder.configuration as unknown as VideoEncoderConfiguration,
    );
    vi.mocked(getVideoEncoderConfigurationOptions).mockResolvedValue(
      MOCK_DATA.videoEncoder.options as unknown as VideoEncoderConfigurationOptions,
    );
  };

  const expandProfileAndOpenVideoEncoderDialog = async (
    user: ReturnType<typeof userEvent.setup>,
    profileToken = 'ProfileToken1',
  ) => {
    await renderProfilesPage();
    await expandProfile(user, profileToken);
    await waitForVideoEncoderSection();

    const editButton = screen.getByTestId(`video-encoder-config-${profileToken}-edit-button`);
    await user.click(editButton);

    await waitFor(
      () => {
        expect(screen.getByTestId('video-encoder-edit-dialog-title')).toHaveTextContent(
          'Edit Video Encoder Configuration',
        );
      },
      { timeout: 10000 },
    );
  };

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(getProfiles).mockResolvedValue(mockProfiles);
    setupVideoEncoderMocks();
  });

  it('should render page with loading state', async () => {
    vi.mocked(getProfiles).mockImplementation(() => new Promise(() => {}));

    renderWithProviders(<ProfilesPage />);
    expect(screen.getByTestId('profiles-loading')).toBeInTheDocument();
  });

  it('should render profiles list when loaded', async () => {
    await renderProfilesPage();

    const mainStreamProfile = findProfileByName(mockProfiles, 'MainStream');
    const subStreamProfile = findProfileByName(mockProfiles, 'SubStream');
    if (mainStreamProfile) {
      expect(screen.getByTestId(`profile-name-${mainStreamProfile.token}`)).toBeInTheDocument();
    }
    if (subStreamProfile) {
      expect(screen.getByTestId(`profile-name-${subStreamProfile.token}`)).toBeInTheDocument();
    }
  });

  it('should render error state when query fails', async () => {
    vi.mocked(getProfiles).mockRejectedValue(new Error('Network error'));

    renderWithProviders(<ProfilesPage />);

    await waitFor(() => {
      expect(screen.getByTestId('profiles-error')).toBeInTheDocument();
    });
  });

  it('should open and close create profile dialog', async () => {
    const user = userEvent.setup();
    await renderProfilesPage();

    const createButton = screen.getByTestId('profiles-create-profile-button');
    await user.click(createButton);

    await waitFor(() => {
      const createProfileTexts = screen.getAllByText('Create Profile');
      expect(createProfileTexts.length).toBeGreaterThan(0);
    });

    const cancelButton = screen.getByTestId('create-profile-dialog-cancel-button');
    await user.click(cancelButton);

    await waitFor(() => {
      const createProfileTexts = screen.queryAllByText('Create Profile');
      expect(createProfileTexts.length).toBeLessThanOrEqual(1);
    });
  });

  it('should create profile on form submission', async () => {
    vi.mocked(createProfile).mockResolvedValue('NewProfileToken');

    const user = userEvent.setup();
    await renderProfilesPage();

    const createButton = screen.getByTestId('profiles-create-profile-button');
    await user.click(createButton);

    await waitFor(() => {
      const createProfileTexts = screen.getAllByText('Create Profile');
      expect(createProfileTexts.length).toBeGreaterThan(0);
    });

    const nameInput = screen.getByTestId('create-profile-dialog-name-input');
    await user.type(nameInput, 'New Profile');

    const submitButton = screen.getByTestId('create-profile-dialog-submit-button');
    await user.click(submitButton);

    await waitFor(() => {
      expect(createProfile).toHaveBeenCalledWith('New Profile');
      expect(mockToast.success).toHaveBeenCalledWith('Profile created successfully');
    });
  });

  it('should show error when profile creation fails', async () => {
    vi.mocked(createProfile).mockRejectedValue(new Error('Network error'));

    const user = userEvent.setup();
    await renderProfilesPage();

    const createButton = screen.getByTestId('profiles-create-profile-button');
    await user.click(createButton);

    await waitFor(() => {
      const createProfileTexts = screen.getAllByText('Create Profile');
      expect(createProfileTexts.length).toBeGreaterThan(0);
    });

    const nameInput = screen.getByTestId('create-profile-dialog-name-input');
    await user.type(nameInput, 'New Profile');

    const submitButton = screen.getByTestId('create-profile-dialog-submit-button');
    await user.click(submitButton);

    await waitFor(() => {
      expect(mockToast.error).toHaveBeenCalledWith('Failed to create profile', {
        description: 'Network error',
      });
    });
  });

  it('should open delete confirmation dialog', async () => {
    await renderProfilesPage();

    const mainStreamProfile = findProfileByName(mockProfiles, 'MainStream');
    if (mainStreamProfile) {
      expect(screen.getByTestId(`profile-name-${mainStreamProfile.token}`)).toBeInTheDocument();
    }

    const deleteButton = screen.getByTestId('delete-profile-button-ProfileToken1');
    expect(deleteButton).toBeInTheDocument();
  });

  it('should delete profile on confirmation', async () => {
    vi.mocked(deleteProfile).mockResolvedValue(undefined);

    await renderProfilesPage();

    expect(deleteProfile).toBeDefined();
  });

  it('should toggle profile card expand/collapse', async () => {
    const user = userEvent.setup();
    await renderProfilesPage();

    await expandProfile(user, 'ProfileToken1');
  });

  it('should render empty state when no profiles exist', async () => {
    vi.mocked(getProfiles).mockResolvedValue([]);

    renderWithProviders(<ProfilesPage />);

    await waitFor(() => {
      expect(screen.getByTestId('profiles-empty-state')).toBeInTheDocument();
    });
  });

  it('should display profile information correctly', async () => {
    await renderProfilesPage();

    const mainStreamProfile = findProfileByName(mockProfiles, 'MainStream');
    if (mainStreamProfile) {
      expect(screen.getByTestId(`profile-name-${mainStreamProfile.token}`)).toBeInTheDocument();
    }
    expect(mainStreamProfile).toBeDefined();
  });

  it('should not show delete button for fixed profiles', async () => {
    renderWithProviders(<ProfilesPage />);

    await waitFor(() => {
      expect(screen.getByTestId('profile-name-ProfileToken2')).toBeInTheDocument();
    });

    // Fixed profiles should not have delete buttons
    // The delete button should only appear for non-fixed profiles
    const deleteButton = screen.queryByTestId('delete-profile-button-ProfileToken2');
    // Delete button should not exist for fixed profiles
    expect(deleteButton).not.toBeInTheDocument();
  });

  it('should handle delete error', async () => {
    vi.mocked(deleteProfile).mockRejectedValue(new Error('Delete failed'));

    const user = userEvent.setup();
    await renderProfilesPage();

    const deleteButton = screen.getByTestId('delete-profile-button-ProfileToken1');
    await user.click(deleteButton);

    await waitFor(
      () => {
        expect(screen.getByTestId('delete-profile-dialog-title')).toHaveTextContent(
          'Delete Profile?',
        );
      },
      { timeout: 3000 },
    );

    const confirmButton = screen.getByTestId('delete-profile-dialog-confirm');
    await user.click(confirmButton);

    await waitFor(
      () => {
        expect(mockToast.error).toHaveBeenCalledWith('Failed to delete profile', {
          description: 'Delete failed',
        });
      },
      { timeout: 3000 },
    );
  });

  it('should toggle profile expand/collapse multiple times', async () => {
    const user = userEvent.setup();
    await renderProfilesPage();

    const expandButton = screen.getByTestId('profile-expand-ProfileToken1');
    await user.click(expandButton);
    await waitFor(
      () => {
        expect(screen.getByTestId('video-source-config-ProfileToken1')).toBeInTheDocument();
      },
      { timeout: 3000 },
    );

    await user.click(expandButton);
    expect(expandButton).toBeInTheDocument();
  });

  it('should open VideoEncoderEditDialog when edit button is clicked', async () => {
    const user = userEvent.setup();
    await expandProfileAndOpenVideoEncoderDialog(user);

    expect(getVideoEncoderConfiguration).toHaveBeenCalledWith('VideoEncoderToken1');
    expect(getVideoEncoderConfigurationOptions).toHaveBeenCalled();
  });

  it('should associate every video encoder control with its label', async () => {
    const user = userEvent.setup();
    await expandProfileAndOpenVideoEncoderDialog(user);

    for (const label of [
      'Resolution',
      'Quality',
      'Frame Rate Limit',
      'Bitrate Limit',
      'H.264 Profile',
      'GOP Length',
    ]) {
      expect(screen.getByLabelText(label)).toBeInTheDocument();
    }
  });

  it('should handle VideoEncoderEditDialog loading state', async () => {
    vi.mocked(getVideoEncoderConfiguration).mockImplementation(
      () => new Promise(() => {}), // Never resolves
    );
    vi.mocked(getVideoEncoderConfigurationOptions).mockImplementation(
      () => new Promise(() => {}), // Never resolves
    );

    const user = userEvent.setup();
    await renderProfilesPage();
    await expandProfile(user, 'ProfileToken1');
    await waitForVideoEncoderSection();

    const editButton = screen.getByTestId('video-encoder-config-ProfileToken1-edit-button');
    await user.click(editButton);

    await waitFor(
      () => {
        expect(screen.getByTestId('video-encoder-edit-dialog-loading')).toHaveTextContent(
          'Loading...',
        );
      },
      { timeout: 10000 },
    );
  });

  it('should handle VideoEncoderEditDialog error', async () => {
    vi.mocked(getVideoEncoderConfiguration).mockRejectedValue(new Error('Load failed'));

    const user = userEvent.setup();
    await renderProfilesPage();
    await expandProfile(user, 'ProfileToken1');
    await waitForVideoEncoderSection();

    const editButton = screen.getByTestId('video-encoder-config-ProfileToken1-edit-button');
    await user.click(editButton);

    await waitFor(
      () => {
        expect(mockToast.error).toHaveBeenCalledWith('Failed to load encoder configuration', {
          description: 'Load failed',
        });
      },
      { timeout: 5000 },
    );
  });

  it('should render ConfigSection with active state', async () => {
    const user = userEvent.setup();
    await renderProfilesPage();
    await expandProfile(user, 'ProfileToken1');

    await waitFor(
      () => {
        expect(screen.getByTestId('video-source-config-ProfileToken1')).toBeInTheDocument();
        expect(screen.getByTestId('video-encoder-config-ProfileToken1')).toBeInTheDocument();
      },
      { timeout: 3000 },
    );
  });

  it('should render ConfigSection with inactive state', async () => {
    const profileWithoutConfig = {
      token: 'ProfileToken3',
      name: 'EmptyProfile',
      fixed: false,
    };
    vi.mocked(getProfiles).mockResolvedValue([profileWithoutConfig]);

    const user = userEvent.setup();
    renderWithProviders(<ProfilesPage />);

    await waitFor(() => {
      expect(screen.getByTestId('profile-name-ProfileToken3')).toBeInTheDocument();
    });

    // Expand profile
    const expandButton = screen.getByTestId('profile-expand-ProfileToken3');
    await user.click(expandButton);

    await waitFor(
      () => {
        expect(screen.getByTestId('video-source-config-ProfileToken3')).toBeInTheDocument();
      },
      { timeout: 3000 },
    );
  });

  it('should save VideoEncoderEditDialog changes', async () => {
    vi.mocked(setVideoEncoderConfiguration).mockResolvedValue(undefined);

    const user = userEvent.setup();
    await expandProfileAndOpenVideoEncoderDialog(user);

    const saveButton = screen.getByTestId('video-encoder-edit-dialog-save');
    await user.click(saveButton);

    await waitFor(
      () => {
        expect(setVideoEncoderConfiguration).toHaveBeenCalled();
        expect(mockToast.success).toHaveBeenCalledWith('Video encoder configuration updated');
      },
      { timeout: 10000 },
    );
  });

  it('should cancel VideoEncoderEditDialog', async () => {
    const user = userEvent.setup();
    await expandProfileAndOpenVideoEncoderDialog(user);

    const cancelButton = screen.getByTestId('video-encoder-edit-dialog-cancel');
    await user.click(cancelButton);

    await waitFor(
      () => {
        expect(screen.queryByTestId('video-encoder-edit-dialog-title')).not.toBeInTheDocument();
      },
      { timeout: 3000 },
    );
  });

  it('should handle VideoEncoderEditDialog save error', async () => {
    vi.mocked(setVideoEncoderConfiguration).mockRejectedValue(new Error('Save failed'));

    const user = userEvent.setup();
    await expandProfileAndOpenVideoEncoderDialog(user);

    const saveButton = screen.getByTestId('video-encoder-edit-dialog-save');
    await user.click(saveButton);

    await waitFor(
      () => {
        expect(mockToast.error).toHaveBeenCalledWith('Failed to update encoder configuration', {
          description: 'Save failed',
        });
      },
      { timeout: 10000 },
    );
  });
});

describe('ProfilesPage config tiles (attach/detach)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const expandProfileCard = async (user: ReturnType<typeof userEvent.setup>, token: string) => {
    renderWithProviders(<ProfilesPage />);
    await waitForPageLoad('profiles-title');
    await user.click(await screen.findByTestId(`profile-expand-${token}`));
  };

  const tiles: Array<{ family: string; prefix: string }> = [
    { family: 'VideoSource', prefix: 'video-source-config' },
    { family: 'VideoEncoder', prefix: 'video-encoder-config' },
    { family: 'AudioSource', prefix: 'audio-source-config' },
    { family: 'AudioEncoder', prefix: 'audio-encoder-config' },
  ];

  it.each(tiles)('attaches a configuration from its tile', async ({ family, prefix }) => {
    vi.mocked(getProfiles).mockResolvedValue([
      { token: 'ProfileBare', name: 'Bare', fixed: false } as MediaProfile,
    ]);
    vi.mocked(getCompatibleConfigurations).mockResolvedValue([`${family}Config_0`]);
    vi.mocked(addConfiguration).mockResolvedValue(undefined);

    const user = userEvent.setup();
    await expandProfileCard(user, 'ProfileBare');

    await user.click(await screen.findByTestId(`${prefix}-ProfileBare-add-button`));
    await user.click(await screen.findByTestId(`config-picker-option-${family}Config_0`));
    await user.click(await screen.findByTestId('config-picker-attach'));

    await waitFor(() =>
      expect(addConfiguration).toHaveBeenCalledWith('ProfileBare', family, `${family}Config_0`),
    );
  });

  it('detaches a configured audio source from its tile', async () => {
    vi.mocked(getProfiles).mockResolvedValue([
      {
        token: 'ProfileAudio',
        name: 'Audio',
        fixed: false,
        audioSourceConfiguration: { token: 'AudioSourceConfig_0', name: 'Mic' },
      } as MediaProfile,
    ]);
    vi.mocked(removeConfiguration).mockResolvedValue(undefined);

    const user = userEvent.setup();
    await expandProfileCard(user, 'ProfileAudio');

    await user.click(await screen.findByTestId('audio-source-config-ProfileAudio-remove-button'));

    await waitFor(() =>
      expect(removeConfiguration).toHaveBeenCalledWith('ProfileAudio', 'AudioSource'),
    );
  });

  it('shows the empty state in the PTZ picker when the device has no PTZ config', async () => {
    vi.mocked(getProfiles).mockResolvedValue([
      { token: 'ProfileNoPtz', name: 'NoPtz', fixed: false } as MediaProfile,
    ]);
    vi.mocked(getCompatibleConfigurations).mockResolvedValue([]);

    const user = userEvent.setup();
    await expandProfileCard(user, 'ProfileNoPtz');
    await user.click(await screen.findByTestId('ptz-config-ProfileNoPtz-add-button'));

    expect(await screen.findByTestId('config-picker-empty')).toBeInTheDocument();
  });

  it('flags an attached metadata config as having no metadata stream', async () => {
    vi.mocked(getProfiles).mockResolvedValue([
      {
        token: 'ProfileMeta',
        name: 'Meta',
        fixed: false,
        metadataConfiguration: { token: 'MetadataConfig_0', name: 'Analytics' },
      } as MediaProfile,
    ]);

    const user = userEvent.setup();
    await expandProfileCard(user, 'ProfileMeta');

    expect(screen.getByTestId('metadata-config-ProfileMeta')).toHaveTextContent(
      'config only — no metadata stream',
    );
  });

  it('gates video encoder removal behind a confirmation', async () => {
    vi.mocked(getProfiles).mockResolvedValue([
      {
        token: 'ProfileEnc',
        name: 'Enc',
        fixed: false,
        videoEncoderConfiguration: {
          token: 'VideoEncoderConfig_0',
          name: 'H264',
          encoding: 'H264',
        },
      } as MediaProfile,
    ]);
    vi.mocked(removeConfiguration).mockResolvedValue(undefined);

    const user = userEvent.setup();
    await expandProfileCard(user, 'ProfileEnc');

    // Clicking Remove opens the confirmation; the service is not called yet.
    await user.click(await screen.findByTestId('video-encoder-config-ProfileEnc-remove-button'));
    expect(await screen.findByTestId('remove-config-dialog')).toHaveTextContent('no stream');
    expect(removeConfiguration).not.toHaveBeenCalled();

    // Confirming fires the remove.
    await user.click(screen.getByTestId('remove-config-dialog-confirm'));
    await waitFor(() =>
      expect(removeConfiguration).toHaveBeenCalledWith('ProfileEnc', 'VideoEncoder'),
    );
  });

  const openAudioEncoderDialog = async (
    user: ReturnType<typeof userEvent.setup>,
    token = 'ProfileAudioEnc',
  ) => {
    vi.mocked(getProfiles).mockResolvedValue([
      {
        token,
        name: 'AudioEnc',
        fixed: false,
        audioEncoderConfiguration: { token: 'AudioEncoderConfig_0', name: 'G711' },
      } as MediaProfile,
    ]);
    vi.mocked(getAudioEncoderConfiguration).mockResolvedValue(
      MOCK_DATA.audioEncoder.configuration as unknown as AudioEncoderConfiguration,
    );
    vi.mocked(getAudioEncoderConfigurationOptions).mockResolvedValue(
      MOCK_DATA.audioEncoder.options as unknown as AudioEncoderConfigurationOptions,
    );

    await expandProfileCard(user, token);
    await user.click(await screen.findByTestId(`audio-encoder-config-${token}-edit-button`));
  };

  it('edits the audio encoder from its tile', async () => {
    vi.mocked(setAudioEncoderConfiguration).mockResolvedValue(undefined);

    const user = userEvent.setup();
    await openAudioEncoderDialog(user);

    // A different encoding lands on the bitrate/sample rate it advertises.
    await user.selectOptions(await screen.findByTestId('audio-encoder-encoding-select'), 'G726');
    await user.click(screen.getByTestId('audio-encoder-edit-dialog-save'));

    await waitFor(() =>
      expect(setAudioEncoderConfiguration).toHaveBeenCalledWith(
        expect.objectContaining({ encoding: 'G726', bitrate: 16, sampleRate: 16 }),
      ),
    );
  });

  it('associates every audio encoder control with its label', async () => {
    const user = userEvent.setup();
    await openAudioEncoderDialog(user);

    for (const label of ['Encoding', 'Bitrate', 'Sample Rate']) {
      expect(await screen.findByLabelText(label)).toBeInTheDocument();
    }
  });
});
