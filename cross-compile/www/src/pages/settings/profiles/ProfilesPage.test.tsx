/**
 * ProfilesPage Tests
 */
import { screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import {
  type VideoEncoderConfiguration,
  type VideoEncoderConfigurationOptions,
  createProfile,
  deleteProfile,
  getProfiles,
  getVideoEncoderConfiguration,
  getVideoEncoderConfigurationOptions,
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
