# Testing Framework - Anyka AK3918 Project

## Rust Testing

### Test Types

| Type | Location | Purpose |
|------|----------|---------|
| Unit Tests | `src/**/*.rs` (inline `mod tests`) | Test individual functions in isolation |
| Integration Tests | `tests/*.rs` | Test public API and module interactions |
| Benchmarks | `benches/` (with criterion) | Performance testing |

### Running Tests

**⚠️ Cross-compile note**: Default target is ARM. Use `--target x86_64-unknown-linux-gnu` for host-side testing.

```bash
cd cross-compile/onvif-rust

# All tests (on host)
cargo test --target x86_64-unknown-linux-gnu

# Unit tests only
cargo test --target x86_64-unknown-linux-gnu --lib

# Specific test
cargo test --target x86_64-unknown-linux-gnu test_device_get_info

# With output
cargo test --target x86_64-unknown-linux-gnu -- --nocapture

# Ignored tests
cargo test --target x86_64-unknown-linux-gnu -- --ignored

# Coverage (requires tarpaulin)
cargo tarpaulin --target x86_64-unknown-linux-gnu --out Html --output-dir coverage
```

### Mocking with mockall

```rust
use mockall::{automock, predicate::*};
use async_trait::async_trait;

// Define trait with automock
#[automock]
#[async_trait]
trait PlatformService {
    async fn get_device_info(&self) -> Result<DeviceInfo, Error>;
    async fn set_brightness(&self, level: u8) -> Result<(), Error>;
}

// Test usage
#[tokio::test]
async fn test_brightness_setting() {
    let mut mock = MockPlatformService::new();
    
    mock.expect_set_brightness()
        .with(eq(75))
        .times(1)
        .returning(|_| Ok(()));
    
    let result = mock.set_brightness(75).await;
    assert!(result.is_ok());
}
```

### Test Naming Convention

```rust
// Pattern: test_<function>_<scenario>_<expected_outcome>
fn test_device_get_info_success() { }
fn test_device_get_info_unauthorized_returns_error() { }
fn test_media_create_profile_invalid_name_returns_validation_error() { }
```

## WebUI Testing

### Test Types

| Type | Tool | Purpose |
|------|------|---------|
| Unit Tests | Vitest | Component logic, hooks, utils |
| Component Tests | Testing Library | UI rendering and interaction |
| API Mocking | MSW | Mock HTTP responses |

### Running Tests

```bash
cd cross-compile/www

# All tests
npm run test

# Watch mode
npm run test -- --watch

# Coverage
npm run test:coverage

# Specific file
npm run test -- ComponentName.test.tsx
```

### Component Test Pattern

```typescript
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { DevicePanel } from './DevicePanel';

describe('DevicePanel', () => {
  it('renders device information correctly', () => {
    render(<DevicePanel deviceId="test-123" />);
    
    // ALWAYS use data-testid for selectors
    expect(screen.getByTestId('device-panel-container')).toBeInTheDocument();
    expect(screen.getByTestId('device-panel-title')).toHaveTextContent('Device');
  });

  it('calls onSave when save button is clicked', async () => {
    const onSave = vi.fn();
    render(<DevicePanel deviceId="test-123" onSave={onSave} />);
    
    await userEvent.click(screen.getByTestId('device-panel-save-button'));
    
    expect(onSave).toHaveBeenCalledTimes(1);
  });
});
```

### MSW for API Mocking

```typescript
import { http, HttpResponse } from 'msw';
import { setupServer } from 'msw/node';

const handlers = [
  http.post('/onvif/device_service', () => {
    return HttpResponse.xml(`
      <SOAP-ENV:Envelope>
        <SOAP-ENV:Body>
          <tds:GetDeviceInformationResponse>
            <tds:Manufacturer>Anyka</tds:Manufacturer>
            <tds:Model>AK3918</tds:Model>
          </tds:GetDeviceInformationResponse>
        </SOAP-ENV:Body>
      </SOAP-ENV:Envelope>
    `);
  }),
];

const server = setupServer(...handlers);

beforeAll(() => server.listen());
afterEach(() => server.resetHandlers());
afterAll(() => server.close());
```

### Test Selector Rules

```typescript
// ❌ WRONG - Do not use these
screen.getByRole('button', { name: 'Save' });
screen.getByText('Save');
screen.getByClassName('btn-primary');

// ✅ CORRECT - Always use data-testid
screen.getByTestId('device-panel-save-button');
screen.getByTestId('user-dialog-cancel-button');
```

## CI Integration

### GitHub Actions

Tests run automatically on:
- Push to `main` branch
- Pull requests to `main`

### Coverage Requirements

| Project | Tool | Target |
|---------|------|--------|
| Rust | tarpaulin | Report to SonarCloud |
| WebUI | v8 coverage | Report to SonarCloud |

### Quality Gates

All tests must pass before merge:
```bash
# Rust (host target for testing)
cargo test --target x86_64-unknown-linux-gnu

# WebUI
npm run test
```
