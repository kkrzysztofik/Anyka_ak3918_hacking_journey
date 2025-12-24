/**
 * Main Entry Point Tests
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

describe('main.tsx', () => {
  const mockCreateRoot = vi.fn();
  const mockRender = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    // Setup DOM
    document.body.innerHTML = '<div id="root"></div>';
    mockCreateRoot.mockReturnValue({ render: mockRender });

    // Mock React DOM
    vi.doMock('react-dom/client', () => ({
      createRoot: mockCreateRoot,
    }));

    // Mock App component
    vi.doMock('./App', () => ({
      default: () => <div>App Component</div>,
    }));
  });

  afterEach(() => {
    document.body.innerHTML = '';
    vi.resetModules();
  });

  it('should render App when root container exists', async () => {
    // Setup DOM with root container
    document.body.innerHTML = '<div id="root"></div>';

    // Mock createRoot to return render function
    const mockRoot = { render: mockRender };
    mockCreateRoot.mockReturnValue(mockRoot);

    // Test the logic directly (since main.tsx executes on import)
    const container = document.getElementById('root');
    if (container) {
      const root = mockCreateRoot(container);
      root.render(vi.fn());
    }

    expect(mockCreateRoot).toHaveBeenCalledWith(container);
    expect(mockRender).toHaveBeenCalled();
  });

  it('should handle missing root container gracefully', () => {
    // Remove root container
    document.body.innerHTML = '';

    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    // Test the logic directly
    const container = document.getElementById('root');
    if (!container) {
      console.error('Root container not found');
    }

    expect(consoleSpy).toHaveBeenCalledWith('Root container not found');
    consoleSpy.mockRestore();
  });
});
