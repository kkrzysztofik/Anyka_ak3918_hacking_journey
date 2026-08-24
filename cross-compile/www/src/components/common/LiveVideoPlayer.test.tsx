/**
 * LiveVideoPlayer Tests
 */
import { waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { renderWithProviders } from '@/test/componentTestHelpers';

import { LiveVideoPlayer } from './LiveVideoPlayer';

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

vi.mock('@/hooks/useAuth', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/hooks/useAuth')>()),
  useAuth: () => ({ getBasicAuthHeader: async () => 'Basic dXNlcjpwYXNz' }),
}));

describe('LiveVideoPlayer', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('creates a player pointed at the requested stream', async () => {
    renderWithProviders(<LiveVideoPlayer streamType="main" />);

    await waitFor(() => expect(createPlayer).toHaveBeenCalled());
    expect(createPlayer.mock.calls[0][0]).toMatchObject({
      type: 'flv',
      isLive: true,
      url: '/live/main.flv',
    });
  });

  it('passes the Basic auth header through to the player', async () => {
    renderWithProviders(<LiveVideoPlayer streamType="main" />);

    await waitFor(() => expect(createPlayer).toHaveBeenCalled());
    expect(createPlayer.mock.calls[0][1]).toMatchObject({
      headers: { Authorization: 'Basic dXNlcjpwYXNz' },
    });
  });

  it('leaves live latency chasing off', async () => {
    renderWithProviders(<LiveVideoPlayer streamType="main" />);

    await waitFor(() => expect(createPlayer).toHaveBeenCalled());
    expect(createPlayer.mock.calls[0][1]).toMatchObject({
      liveBufferLatencyChasing: false,
      enableStashBuffer: false,
    });
  });

  it('destroys the player on unmount so the connection is released', async () => {
    const { unmount } = renderWithProviders(<LiveVideoPlayer streamType="main" />);

    await waitFor(() => expect(createPlayer).toHaveBeenCalled());
    unmount();
    await waitFor(() => expect(mockPlayer.destroy).toHaveBeenCalledTimes(1));
  });

  it('destroys and recreates the player when the stream type changes', async () => {
    const { rerender } = renderWithProviders(<LiveVideoPlayer streamType="main" />);
    await waitFor(() => expect(createPlayer).toHaveBeenCalledTimes(1));

    rerender(<LiveVideoPlayer streamType="sub" />);

    await waitFor(() => expect(mockPlayer.destroy).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(createPlayer).toHaveBeenCalledTimes(2));
    expect(createPlayer.mock.calls[1][0]).toMatchObject({ url: '/live/sub.flv' });
  });

  it('does not recreate the player when only a callback identity changes', async () => {
    const { rerender } = renderWithProviders(
      <LiveVideoPlayer streamType="main" onStateChange={() => {}} />,
    );
    await waitFor(() => expect(createPlayer).toHaveBeenCalledTimes(1));

    rerender(<LiveVideoPlayer streamType="main" onStateChange={() => {}} />);

    expect(createPlayer).toHaveBeenCalledTimes(1);
    expect(mockPlayer.destroy).not.toHaveBeenCalled();
  });

  it('reports connecting before the player is ready', () => {
    const onStateChange = vi.fn();
    renderWithProviders(<LiveVideoPlayer streamType="main" onStateChange={onStateChange} />);

    expect(onStateChange).toHaveBeenCalledWith('connecting');
  });

  it('surfaces a credentials-specific message on a 401', async () => {
    const onStateChange = vi.fn();
    renderWithProviders(<LiveVideoPlayer streamType="main" onStateChange={onStateChange} />);
    await waitFor(() => expect(mockPlayer.on).toHaveBeenCalled());

    const errorHandler = mockPlayer.on.mock.calls.find((c) => c[0] === 'error')?.[1];
    errorHandler('NetworkError', 'HttpStatusCodeInvalid', { code: 401 });

    expect(onStateChange).toHaveBeenCalledWith(
      'error',
      'The camera rejected these credentials for the video stream.',
    );
  });

  it('surfaces a setup failure as an error state instead of an unhandled rejection', async () => {
    const onStateChange = vi.fn();
    createPlayer.mockImplementationOnce(() => {
      throw new Error('player construction failed');
    });
    renderWithProviders(<LiveVideoPlayer streamType="main" onStateChange={onStateChange} />);

    await waitFor(() =>
      expect(onStateChange).toHaveBeenCalledWith(
        'error',
        expect.stringContaining('player construction failed'),
      ),
    );
  });

  it('forwards decoded media info to onStats', async () => {
    const onStats = vi.fn();
    renderWithProviders(<LiveVideoPlayer streamType="main" onStats={onStats} />);
    await waitFor(() => expect(mockPlayer.on).toHaveBeenCalled());

    const mediaHandler = mockPlayer.on.mock.calls.find((c) => c[0] === 'media_info')?.[1];
    mediaHandler({ width: 1280, height: 720, fps: 25, videoCodec: 'avc1.4d001f' });

    expect(onStats).toHaveBeenCalledWith(expect.objectContaining({ width: 1280, height: 720 }));
  });

  it('formats the raw AVC codec tag into a human label', async () => {
    const onStats = vi.fn();
    renderWithProviders(<LiveVideoPlayer streamType="main" onStats={onStats} />);
    await waitFor(() => expect(mockPlayer.on).toHaveBeenCalled());

    const mediaHandler = mockPlayer.on.mock.calls.find((c) => c[0] === 'media_info')?.[1];
    mediaHandler({ width: 1280, height: 720, fps: 25, videoCodec: 'avc1.4de028' });

    expect(onStats).toHaveBeenCalledWith(
      expect.objectContaining({ videoCodec: 'H.264 Main@L4.0' }),
    );
  });

  it('forwards audio track details to onStats', async () => {
    const onStats = vi.fn();
    renderWithProviders(<LiveVideoPlayer streamType="main" onStats={onStats} />);
    await waitFor(() => expect(mockPlayer.on).toHaveBeenCalled());

    const mediaHandler = mockPlayer.on.mock.calls.find((c) => c[0] === 'media_info')?.[1];
    mediaHandler({
      width: 1280,
      height: 720,
      videoCodec: 'avc1.4de028',
      audioCodec: 'mp4a.40.2',
      audioSampleRate: 8000,
      audioChannelCount: 1,
    });

    expect(onStats).toHaveBeenCalledWith(
      expect.objectContaining({
        audioCodec: 'AAC-LC',
        audioSampleRate: 8000,
        audioChannels: 1,
      }),
    );
  });

  it('starts muted so autoplay is allowed, and offers no toggle without audio', async () => {
    // Autoplay policies block a non-muted element from starting on its own, so
    // an unmuted default would leave the preview stuck rather than noisy.
    const { getByTestId, queryByTestId } = renderWithProviders(
      <LiveVideoPlayer streamType="main" />,
    );
    await waitFor(() => expect(mockPlayer.on).toHaveBeenCalled());

    expect((getByTestId('liveview-video') as HTMLVideoElement).muted).toBe(true);
    expect(queryByTestId('liveview-mute-toggle')).toBeNull();
  });

  it('unmutes the element when the toggle is clicked on a stream with audio', async () => {
    const { getByTestId } = renderWithProviders(<LiveVideoPlayer streamType="main" />);
    await waitFor(() => expect(mockPlayer.on).toHaveBeenCalled());

    const mediaHandler = mockPlayer.on.mock.calls.find((c) => c[0] === 'media_info')?.[1];
    mediaHandler({ width: 1280, height: 720, videoCodec: 'avc1.4de028', audioCodec: 'mp4a.40.2' });

    const toggle = await waitFor(() => getByTestId('liveview-mute-toggle'));
    const video = getByTestId('liveview-video') as HTMLVideoElement;
    expect(video.muted).toBe(true);

    toggle.click();
    await waitFor(() => expect(video.muted).toBe(false));

    toggle.click();
    await waitFor(() => expect(video.muted).toBe(true));
  });

  it('does not trust the SPS-derived fps from MEDIA_INFO', async () => {
    const onStats = vi.fn();
    renderWithProviders(<LiveVideoPlayer streamType="main" onStats={onStats} />);
    await waitFor(() => expect(mockPlayer.on).toHaveBeenCalled());

    const mediaHandler = mockPlayer.on.mock.calls.find((c) => c[0] === 'media_info')?.[1];
    mediaHandler({ width: 1280, height: 720, fps: 23.976, videoCodec: 'avc1.4de028' });

    expect(onStats).toHaveBeenCalledWith(expect.not.objectContaining({ fps: expect.anything() }));
  });

  it('measures real fps from decoded frame deltas', async () => {
    const onStats = vi.fn();
    renderWithProviders(<LiveVideoPlayer streamType="main" onStats={onStats} />);
    await waitFor(() => expect(mockPlayer.on).toHaveBeenCalled());

    const now = vi.spyOn(performance, 'now');
    const statsHandler = mockPlayer.on.mock.calls.find((c) => c[0] === 'statistics_info')?.[1];

    now.mockReturnValue(0);
    statsHandler({ speed: 128, decodedFrames: 0, droppedFrames: 0 });
    now.mockReturnValue(1000);
    statsHandler({ speed: 128, decodedFrames: 15, droppedFrames: 0 });

    expect(onStats).toHaveBeenLastCalledWith(expect.objectContaining({ fps: 15 }));
    now.mockRestore();
  });

  it('keeps fps undefined until two decoded frame samples exist', async () => {
    const onStats = vi.fn();
    renderWithProviders(<LiveVideoPlayer streamType="main" onStats={onStats} />);
    await waitFor(() => expect(mockPlayer.on).toHaveBeenCalled());

    const statsHandler = mockPlayer.on.mock.calls.find((c) => c[0] === 'statistics_info')?.[1];
    statsHandler({ speed: 128, decodedFrames: 0, droppedFrames: 0 });

    expect(onStats).toHaveBeenLastCalledWith(
      expect.not.objectContaining({ fps: expect.anything() }),
    );
  });

  it('converts the reported speed from KB/s to Kbps', async () => {
    const onStats = vi.fn();
    renderWithProviders(<LiveVideoPlayer streamType="main" onStats={onStats} />);
    await waitFor(() => expect(mockPlayer.on).toHaveBeenCalled());

    const statsHandler = mockPlayer.on.mock.calls.find((c) => c[0] === 'statistics_info')?.[1];
    statsHandler({ speed: 128, droppedFrames: 3 });

    expect(onStats).toHaveBeenCalledWith(
      expect.objectContaining({ bitrateKbps: 1024, droppedFrames: 3 }),
    );
  });
});
