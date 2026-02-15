# MSW (Mock Service Worker) Patterns Reference

## Handler Organization

```typescript
// src/mocks/handlers/device.ts
import { http, HttpResponse } from 'msw';

export const deviceHandlers = [
  http.post('/onvif/device_service', ({ request }) => {
    const body = request.body as string;

    if (body.includes('GetDeviceInformation')) {
      return HttpResponse.xml(
        `<?xml version="1.0"?>
<Envelope>
  <Body>
    <GetDeviceInformationResponse>
      <Manufacturer>Anyka</Manufacturer>
      <Model>AK3918</Model>
    </GetDeviceInformationResponse>
  </Body>
</Envelope>`
      );
    }

    if (body.includes('SetHostname')) {
      return HttpResponse.xml(
        `<?xml version="1.0"?>
<Envelope>
  <Body>
    <SetHostnameResponse/>
  </Body>
</Envelope>`
      );
    }

    return HttpResponse.xml(`<Fault>Unsupported</Fault>`, { status: 500 });
  }),
];

// src/mocks/handlers/media.ts
export const mediaHandlers = [
  http.post('/onvif/media_service', ({ request }) => {
    const body = request.body as string;

    if (body.includes('GetProfiles')) {
      return HttpResponse.xml(
        `<?xml version="1.0"?>
<Envelope>
  <Body>
    <GetProfilesResponse>
      <Profiles token="profile1">
        <Name>Profile 1</Name>
      </Profiles>
    </GetProfilesResponse>
  </Body>
</Envelope>`
      );
    }

    return HttpResponse.xml(`<Fault>Unsupported</Fault>`, { status: 500 });
  }),
];

// src/mocks/handlers/index.ts
import { deviceHandlers } from './device';
import { mediaHandlers } from './media';

export const handlers = [...deviceHandlers, ...mediaHandlers];
```

## Dynamic Handler Configuration

```typescript
// src/mocks/server.ts
import { setupServer } from 'msw/node';
import { handlers } from './handlers';

export const server = setupServer(...handlers);

// Reset between tests but keep listeners
export function resetHandlers() {
  server.resetHandlers();
}

// Override specific handler for a test
export function mockGetDeviceInfoFailure() {
  server.use(
    http.post('/onvif/device_service', ({ request }) => {
      const body = request.body as string;
      if (body.includes('GetDeviceInformation')) {
        return HttpResponse.xml(
          `<?xml version="1.0"?>
<Envelope>
  <Body>
    <Fault>
      <Code>
        <Value>Sender</Value>
        <Subcode>
          <Value>ServerBusy</Value>
        </Subcode>
      </Code>
      <Reason>Camera is busy</Reason>
    </Fault>
  </Body>
</Envelope>`,
          { status: 503 }
        );
      }
    })
  );
}

export function mockSetHostnameSuccess() {
  server.use(
    http.post('/onvif/device_service', ({ request }) => {
      const body = request.body as string;
      if (body.includes('SetHostname')) {
        return HttpResponse.xml(
          `<?xml version="1.0"?>
<Envelope>
  <Body>
    <SetHostnameResponse/>
  </Body>
</Envelope>`
        );
      }
    })
  );
}
```

## Test Setup

```typescript
// vitest.config.ts
import { defineConfig } from 'vitest/config';
import path from 'path';

export default defineConfig({
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: ['./src/mocks/setup.ts'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      exclude: [
        'node_modules/',
        'src/mocks/',
      ],
    },
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
});

// src/mocks/setup.ts
import { server } from './server';
import { afterAll, afterEach, beforeAll } from 'vitest';

// Start MSW server
beforeAll(() => server.listen());

// Reset handlers between tests
afterEach(() => server.resetHandlers());

// Cleanup
afterAll(() => server.close());
```

## Request/Response Patterns

### XML Response Building

```typescript
export function buildSoapResponse(content: string): string {
  return `<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope"
            xmlns:tt="http://www.onvif.org/ver10/schema"
            xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
    <s:Body>
        ${content}
    </s:Body>
</s:Envelope>`;
}

export function buildFaultResponse(
  code: string,
  reason: string
): string {
  return buildSoapResponse(`
    <s:Fault>
        <s:Code>
            <s:Value>s:Sender</s:Value>
            <s:Subcode>
                <s:Value xmlns:ter="http://www.onvif.org/ver10/error">ter:${code}</s:Value>
            </s:Subcode>
        </s:Code>
        <s:Reason>
            <s:Text>${reason}</s:Text>
        </s:Reason>
    </s:Fault>
  `);
}

export const soapResponses = {
  deviceInfo: buildSoapResponse(`
    <tds:GetDeviceInformationResponse>
        <tds:DeviceInformation>
            <tt:Manufacturer>Anyka</tt:Manufacturer>
            <tt:Model>AK3918</tt:Model>
            <tt:FirmwareVersion>1.0.0</tt:FirmwareVersion>
            <tt:SerialNumber>SN123456</tt:SerialNumber>
            <tt:HardwareId>HW001</tt:HardwareId>
        </tds:DeviceInformation>
    </tds:GetDeviceInformationResponse>
  `),

  notAuthorized: buildFaultResponse('NotAuthorized', 'Not authorized'),
  invalidArgs: buildFaultResponse('InvalidArgVal', 'Invalid argument'),
  serverError: buildFaultResponse('Action', 'Internal server error'),
};
```

### Conditional Responses

```typescript
http.post('/onvif/device_service', ({ request }) => {
  const body = request.body as string;

  // Inspect request to determine response
  if (body.includes('InvalidDeviceId')) {
    return HttpResponse.xml(soapResponses.invalidArgs, { status: 400 });
  }

  if (body.includes('UnauthorizedToken')) {
    return HttpResponse.xml(
      soapResponses.notAuthorized,
      { status: 401 }
    );
  }

  // Normal response
  return HttpResponse.xml(soapResponses.deviceInfo);
}),
```

## Delay Simulation

```typescript
// Test slow network
http.post('/onvif/device_service', async ({ request }) => {
  // Simulate 2 second delay
  await new Promise((resolve) => setTimeout(resolve, 2000));

  return HttpResponse.xml(soapResponses.deviceInfo);
}),

// Test timeout
http.post('/onvif/media_service', async ({ request }) => {
  // Never responds - triggers timeout
  await new Promise(() => {});
}),
```

## Test-Specific Overrides

```typescript
describe('DeviceSettings', () => {
  afterEach(() => {
    server.resetHandlers();
  });

  it('handles successful save', async () => {
    // All handlers work normally
    render(<DeviceSettings />, { wrapper: createWrapper() });
    // ... test ...
  });

  it('handles save failure', async () => {
    // Override handler for this test only
    server.use(
      http.post('/onvif/device_service', () => {
        return HttpResponse.xml(soapResponses.serverError, { status: 500 });
      })
    );

    render(<DeviceSettings />, { wrapper: createWrapper() });
    // ... test error handling ...
  });

  it('handles network timeout', async () => {
    server.use(
      http.post('/onvif/device_service', async () => {
        // Simulate infinite delay
        await new Promise(() => {});
      })
    );

    render(<DeviceSettings />, { wrapper: createWrapper() });
    // ... test timeout handling ...
  });
});
```

## Stateful Handlers

```typescript
export const statefulHandlers = [
  http.post('/onvif/device_service', (() => {
    let hostname = 'camera-default';

    return ({ request }) => {
      const body = request.body as string;

      if (body.includes('GetHostname')) {
        return HttpResponse.xml(buildSoapResponse(`
          <tds:GetHostnameResponse>
              <tds:HostnameInformation>
                  <tt:Name>${hostname}</tt:Name>
              </tds:HostnameInformation>
          </tds:GetHostnameResponse>
        `));
      }

      if (body.includes('SetHostname')) {
        // Extract hostname from request
        const match = body.match(/<tt:Name>([^<]+)<\/tt:Name>/);
        if (match) {
          hostname = match[1];
        }

        return HttpResponse.xml(buildSoapResponse(
          '<tds:SetHostnameResponse/>'
        ));
      }
    };
  })()),
];
```

## Error Scenarios

```typescript
export const errorScenarios = {
  unauthorized: () =>
    server.use(
      http.post('/onvif/device_service', () =>
        HttpResponse.xml(soapResponses.notAuthorized, { status: 401 })
      )
    ),

  forbidden: () =>
    server.use(
      http.post('/onvif/device_service', () =>
        HttpResponse.xml(buildFaultResponse('NotAllowed', 'Operation not allowed'), {
          status: 403,
        })
      )
    ),

  notFound: () =>
    server.use(
      http.post('/onvif/device_service', () =>
        HttpResponse.xml(buildFaultResponse('NoDevice', 'Device not found'), {
          status: 404,
        })
      )
    ),

  serverError: () =>
    server.use(
      http.post('/onvif/device_service', () =>
        HttpResponse.xml(soapResponses.serverError, { status: 500 })
      )
    ),

  serviceUnavailable: () =>
    server.use(
      http.post('/onvif/device_service', () =>
        HttpResponse.xml(
          buildFaultResponse('ServiceUnavailable', 'Service temporarily unavailable'),
          { status: 503 }
        )
      )
    ),

  networkError: () =>
    server.use(
      http.post('/onvif/device_service', () =>
        HttpResponse.error()
      )
    ),
};

// Usage in tests
it('handles authentication failure', async () => {
  errorScenarios.unauthorized();
  // ... test ...
});
```

## Integration Testing Pattern

```typescript
describe('User management flow', () => {
  it('creates, edits, and deletes user', async () => {
    const user = userEvent.setup();
    const { rerender } = render(<UserManagement />, {
      wrapper: createWrapper(),
    });

    // 1. Create user
    await user.click(screen.getByTestId('create-user-button'));
    await user.type(screen.getByTestId('user-username-input'), 'newuser');
    await user.click(screen.getByTestId('user-create-button'));

    // MSW stores state - next GET should show new user
    rerender(<UserManagement />);

    // 2. Verify user appears in list
    await waitFor(() => {
      expect(screen.getByText('newuser')).toBeInTheDocument();
    });

    // 3. Edit user
    await user.click(screen.getByTestId('edit-user-newuser'));
    // ... edit form ...

    // 4. Delete user
    await user.click(screen.getByTestId('delete-user-newuser'));
    await user.click(screen.getByTestId('confirm-delete'));

    // Verify deleted
    await waitFor(() => {
      expect(screen.queryByText('newuser')).not.toBeInTheDocument();
    });
  });
});
```
