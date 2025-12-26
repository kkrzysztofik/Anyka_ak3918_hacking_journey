/**
 * Main Entry Point Tests
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

// Mock react-dom/client before any imports
const mockRender = vi.fn();
const mockCreateRoot = vi.fn(() => ({ render: mockRender }));

vi.mock('react-dom/client', () => ({
  createRoot: mockCreateRoot,
}));

// Mock App component
vi.mock('./App', () => ({
  default: () => <div data-testid="app">App Component</div>,
}));

// Mock React.createElement
const mockCreateElement = vi.fn((type, props, ...children) => {
  if (typeof type === 'function') {
    return type(props);
  }
  return { type, props, children };
});

vi.mock('react', () => ({
  default: {
    createElement: mockCreateElement,
  },
}));

describe('main.tsx', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    document.body.innerHTML = '';
  });

  afterEach(() => {
    document.body.innerHTML = '';
    vi.resetModules();
  });

  it('should render App when root container exists', async () => {
    // Setup DOM with root container
    document.body.innerHTML = '<div id="root"></div>';

    // Reset modules to ensure fresh import
    vi.resetModules();

    // Dynamically import main.tsx to execute it
    await import('./main');

    // Wait a tick for async operations
    await new Promise((resolve) => setTimeout(resolve, 0));

    // Verify createRoot was called with the container
    const container = document.getElementById('root');
    expect(mockCreateRoot).toHaveBeenCalledWith(container);
    expect(mockRender).toHaveBeenCalled();
    expect(mockCreateElement).toHaveBeenCalled();
  });

  it('should handle missing root container gracefully', async () => {
    // Ensure no root container exists
    document.body.innerHTML = '';

    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

    // Reset modules to ensure fresh import
    vi.resetModules();

    // Dynamically import main.tsx to execute it
    await import('./main');

    // Wait a tick for async operations
    await new Promise((resolve) => setTimeout(resolve, 0));

    // Verify error was logged
    expect(consoleSpy).toHaveBeenCalledWith('Root container not found');
    // Verify createRoot was not called
    expect(mockCreateRoot).not.toHaveBeenCalled();

    consoleSpy.mockRestore();
  });
});
