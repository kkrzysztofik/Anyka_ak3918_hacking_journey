---
name: anyka-webui-testing
description: |
  WebUI component testing specialist using Vitest, React Testing Library, and MSW for the Camera WebUI.
  Triggers on: "component test", "MSW", "data-testid", "React test", "mock API", "render component".
version: 1.0.0
---

# Camera WebUI Component Testing

Write comprehensive tests for React components using Vitest, React Testing Library, and MSW for API mocking. Follow the project's testing standards and always use `data-testid` for element selection.

## Testing Framework Setup

The project uses:
- **Vitest**: Test runner (same speed as Jest, better ESM support)
- **React Testing Library**: Component rendering and user interaction testing
- **MSW**: Mock Service Worker for API mocking
- **@testing-library/user-event**: User interaction simulation

### Run Tests

```bash
cd cross-compile/www
npm run test                          # Run all tests
npm run test -- DevicePanel           # Run specific test
npm run test -- --watch               # Watch mode
npm run test -- --coverage            # Coverage report
```

## Component Test Template

```typescript
// src/components/DevicePanel.test.tsx

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { DevicePanel } from './DevicePanel';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

// Create a test wrapper with all providers
const createWrapper = () => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>
      {children}
    </QueryClientProvider>
  );
};

describe('DevicePanel', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders device panel with correct title', () => {
    render(<DevicePanel deviceId="test-123" />, {
      wrapper: createWrapper(),
    });

    // ALWAYS use data-testid for selection
    expect(screen.getByTestId('device-panel')).toBeInTheDocument();
  });

  it('displays loading state while fetching', () => {
    render(<DevicePanel deviceId="test-123" />, {
      wrapper: createWrapper(),
    });

    expect(screen.getByTestId('device-panel-loading')).toBeInTheDocument();
  });

  it('displays device information after loading', async () => {
    render(<DevicePanel deviceId="test-123" />, {
      wrapper: createWrapper(),
    });

    // Wait for data to load
    await waitFor(() => {
      expect(screen.queryByTestId('device-panel-loading')).not.toBeInTheDocument();
    });

    // Verify content is displayed
    expect(screen.getByTestId('device-panel-name')).toHaveTextContent('Test Camera');
  });

  it('calls onSave when save button is clicked', async () => {
    const onSave = vi.fn();
    const user = userEvent.setup();

    render(<DevicePanel deviceId="test-123" onSave={onSave} />, {
      wrapper: createWrapper(),
    });

    // Wait for form to load
    await waitFor(() => {
      expect(screen.getByTestId('device-panel-name-input')).toBeInTheDocument();
    });

    // Interact with component
    const saveButton = screen.getByTestId('device-panel-save-button');
    await user.click(saveButton);

    // Verify callback
    expect(onSave).toHaveBeenCalledTimes(1);
    expect(onSave).toHaveBeenCalledWith(expect.objectContaining({
      name: expect.any(String),
    }));
  });

  it('displays error message when API fails', async () => {
    // MSW will handle error response
    render(<DevicePanel deviceId="invalid-id" />, {
      wrapper: createWrapper(),
    });

    await waitFor(() => {
      expect(screen.getByTestId('device-panel-error')).toBeInTheDocument();
    });

    expect(screen.getByTestId('device-panel-error')).toHaveTextContent(
      'Failed to load device information'
    );
  });
});
```

## Selector Priority (MANDATORY)

Always use `data-testid` for test selectors. Never use role or text selectors in this project:

```typescript
// ✅ CORRECT - Always use data-testid
screen.getByTestId('device-panel-save-button');
screen.queryByTestId('error-message');
screen.findByTestId('loading-spinner');

// ❌ WRONG - These are not allowed
screen.getByRole('button', { name: 'Save' });
screen.getByText('Save');
screen.getByClass('btn-primary');
```

## User Interaction Testing

```typescript
it('handles form input and submission', async () => {
  const user = userEvent.setup();

  render(<DeviceSettingsForm />, { wrapper: createWrapper() });

  // Type into input
  const nameInput = screen.getByTestId('device-settings-name-input');
  await user.clear(nameInput);
  await user.type(nameInput, 'My Camera');

  // Select dropdown
  const roleSelect = screen.getByTestId('device-settings-role-select');
  await user.selectOption(roleSelect, 'admin');

  // Check checkbox
  const enabledCheckbox = screen.getByTestId('device-settings-enabled-checkbox');
  await user.click(enabledCheckbox);

  // Submit form
  const submitButton = screen.getByTestId('device-settings-submit-button');
  await user.click(submitButton);

  // Verify request was made
  expect(mockOnSubmit).toHaveBeenCalledWith({
    name: 'My Camera',
    role: 'admin',
    enabled: true,
  });
});
```

## API Mocking with MSW

```typescript
// src/mocks/handlers.ts

import { http, HttpResponse } from 'msw';

export const handlers = [
  // Device service - GetDeviceInformation
  http.post('/onvif/device_service', ({ request }) => {
    const soapBody = request.body as string;

    if (soapBody.includes('GetDeviceInformation')) {
      return HttpResponse.xml(`<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"
            xmlns:tds="http://www.onvif.org/ver10/device/wsdl"
            xmlns:tt="http://www.onvif.org/ver10/schema">
    <s:Body>
        <tds:GetDeviceInformationResponse>
            <tds:DeviceInformation>
                <tt:Manufacturer>Anyka</tt:Manufacturer>
                <tt:Model>AK3918</tt:Model>
                <tt:FirmwareVersion>1.0.0</tt:FirmwareVersion>
                <tt:SerialNumber>SN123456</tt:SerialNumber>
                <tt:HardwareId>HW001</tt:HardwareId>
            </tds:DeviceInformation>
        </tds:GetDeviceInformationResponse>
    </s:Body>
</s:Envelope>`);
    }

    return HttpResponse.xml(`<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
    <s:Body>
        <s:Fault>
            <s:Code>
                <s:Value>s:Sender</s:Value>
                <s:Subcode>
                    <s:Value xmlns:ter="http://www.onvif.org/ver10/error">ter:ActionNotSupported</s:Value>
                </s:Subcode>
            </s:Code>
            <s:Reason>
                <s:Text>Unsupported operation</s:Text>
            </s:Reason>
        </s:Fault>
    </s:Body>
</s:Envelope>`, { status: 500 });
  }),

  // Media service - GetProfiles
  http.post('/onvif/media_service', ({ request }) => {
    const soapBody = request.body as string;

    if (soapBody.includes('GetProfiles')) {
      return HttpResponse.xml(`<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"
            xmlns:trt="http://www.onvif.org/ver10/media/wsdl"
            xmlns:tt="http://www.onvif.org/ver10/schema">
    <s:Body>
        <trt:GetProfilesResponse>
            <trt:Profiles token="profile1" fixed="true">
                <tt:Name>Default Profile</tt:Name>
            </trt:Profiles>
        </trt:GetProfilesResponse>
    </s:Body>
</s:Envelope>`);
    }

    return HttpResponse.xml(`<s:Fault>Unsupported</s:Fault>`, { status: 500 });
  }),
];
```

### Setup MSW in Tests

```typescript
// src/mocks/setup.ts

import { setupServer } from 'msw/node';
import { handlers } from './handlers';

export const server = setupServer(...handlers);

// vitest.config.ts - Configure in setup files
export default defineConfig({
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: ['./src/mocks/setup.ts'],
  },
});

// In test file
beforeAll(() => server.listen());
afterEach(() => server.resetHandlers());
afterAll(() => server.close());
```

## Testing Async Operations

```typescript
it('handles async data loading and updates', async () => {
  render(<UserList />, { wrapper: createWrapper() });

  // Wait for initial load
  await waitFor(() => {
    expect(screen.queryByTestId('users-loading')).not.toBeInTheDocument();
  });

  expect(screen.getByTestId('user-list')).toBeInTheDocument();
  const items = screen.getAllByTestId(/^user-item-/);
  expect(items).toHaveLength(3);
});

it('refetches data when refresh is clicked', async () => {
  const user = userEvent.setup();

  render(<DeviceSettings />, { wrapper: createWrapper() });

  // Initial load
  await waitFor(() => {
    expect(screen.getByTestId('device-settings-loaded')).toBeInTheDocument();
  });

  // Click refresh
  const refreshButton = screen.getByTestId('device-settings-refresh-button');
  await user.click(refreshButton);

  // Should show loading
  expect(screen.getByTestId('device-settings-loading')).toBeInTheDocument();

  // Wait for completion
  await waitFor(() => {
    expect(screen.queryByTestId('device-settings-loading')).not.toBeInTheDocument();
  });
});
```

## Testing Forms and Validation

```typescript
it('validates form inputs before submission', async () => {
  const user = userEvent.setup();

  render(<UserForm />, { wrapper: createWrapper() });

  const submitButton = screen.getByTestId('user-form-submit-button');

  // Submit without filling required fields
  await user.click(submitButton);

  // Verify validation errors
  expect(screen.getByTestId('user-form-username-error')).toHaveTextContent(
    'Username is required'
  );
  expect(screen.getByTestId('user-form-password-error')).toHaveTextContent(
    'Password is required'
  );

  // Fill in fields
  await user.type(
    screen.getByTestId('user-form-username-input'),
    'testuser'
  );
  await user.type(
    screen.getByTestId('user-form-password-input'),
    'password123'
  );

  // Verify errors clear
  await waitFor(() => {
    expect(screen.queryByTestId('user-form-username-error')).not.toBeInTheDocument();
  });

  // Submit should work now
  await user.click(submitButton);
  expect(mockOnSubmit).toHaveBeenCalledTimes(1);
});
```

## Testing Error States

```typescript
it('displays error boundary fallback on error', () => {
  // Mock component to throw
  const { rerender } = render(
    <ErrorBoundary fallback={<div data-testid="error-fallback">Error</div>}>
      <ComponentThatWorks />
    </ErrorBoundary>
  );

  expect(screen.queryByTestId('error-fallback')).not.toBeInTheDocument();

  // Rerender with error
  rerender(
    <ErrorBoundary fallback={<div data-testid="error-fallback">Error</div>}>
      <ComponentThatThrows />
    </ErrorBoundary>
  );

  expect(screen.getByTestId('error-fallback')).toBeInTheDocument();
});
```

## Testing Dialogs and Modals

```typescript
it('opens and closes dialogs correctly', async () => {
  const user = userEvent.setup();

  render(<SettingsPage />, { wrapper: createWrapper() });

  // Dialog should not be visible initially
  expect(screen.queryByTestId('user-dialog')).not.toBeInTheDocument();

  // Click button to open
  await user.click(screen.getByTestId('settings-add-user-button'));

  // Dialog should be visible
  await waitFor(() => {
    expect(screen.getByTestId('user-dialog')).toBeInTheDocument();
  });

  // Fill form and save
  await user.type(
    screen.getByTestId('user-dialog-username-input'),
    'newuser'
  );
  await user.click(screen.getByTestId('user-dialog-save-button'));

  // Dialog should close
  await waitFor(() => {
    expect(screen.queryByTestId('user-dialog')).not.toBeInTheDocument();
  });
});
```

## Reference

For detailed test patterns and MSW setup, see `references/msw-patterns.md`.
