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
    errorHandler('NetworkError', 'Unexpected status 401');

    expect(onStateChange).toHaveBeenCalledWith(
      'error',
      'The camera rejected these credentials for the video stream.',
    );
  });

  it('forwards decoded media info to onStats', async () => {
    const onStats = vi.fn();
    renderWithProviders(<LiveVideoPlayer streamType="main" onStats={onStats} />);
    await waitFor(() => expect(mockPlayer.on).toHaveBeenCalled());

    const mediaHandler = mockPlayer.on.mock.calls.find((c) => c[0] === 'media_info')?.[1];
    mediaHandler({ width: 1280, height: 720, fps: 25, videoCodec: 'avc1.4d001f' });

    expect(onStats).toHaveBeenCalledWith(
      expect.objectContaining({ width: 1280, height: 720, fps: 25 }),
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
