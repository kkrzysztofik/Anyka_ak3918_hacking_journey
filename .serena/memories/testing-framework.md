# Testing Framework - Anyka AK3918 Project

## Rust Testing

### Test Types

| Type | Location | Purpose |
|------|----------|---------|
| Unit Tests | `src/**/*.rs` (inline `mod tests`) | Test individual functions in isolation |
| Integration Tests | `tests/*.rs` | Test public API and module interactions |
| Benchmarks | `benches/` (with criterion) | Performance testing |

### Running Tests

**⚠️ Cross-compile note**: Default target is ARM. Use `--target x86_64-unknown-linux-gnu` for host-side testing. Load the vendored toolchain first with `source ./setenv.sh` from the repo root (exports `$CARGO`, `$RUSTC`, `$RUSTDOC`). Never use bare `cargo`.

The Rust project is a **workspace** (onvif-rust + streaming-lib + anyka-init). Commands from `cross-compile/` run across all members.

```bash
cd cross-compile

# All workspace tests (on host)
$CARGO test --target x86_64-unknown-linux-gnu

# Specific workspace member
$CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust
$CARGO test --target x86_64-unknown-linux-gnu -p streaming-lib

# Unit tests only
$CARGO test --target x86_64-unknown-linux-gnu --lib

# Specific test
$CARGO test --target x86_64-unknown-linux-gnu test_device_get_info

# With output
$CARGO test --target x86_64-unknown-linux-gnu -- --nocapture

# Ignored tests
$CARGO test --target x86_64-unknown-linux-gnu -- --ignored

# Coverage (cargo-llvm-cov; install: cargo install cargo-llvm-cov --locked)
$CARGO llvm-cov --target x86_64-unknown-linux-gnu --workspace \
  --ignore-filename-regex '(/xiu/|/patches/|/anyka_reference/|/onvif/)' \
  --cobertura --output-path coverage/cobertura.xml
```

### Mocking with mockall

```rust
use mockall::{automock, predicate::*};
use async_trait::async_trait;

// Define trait with automock (only in test builds)
#[cfg_attr(test, automock)]
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

This is the project standard (`#[cfg_attr(test, automock)]` on trait definitions, as in `src/platform/common/traits.rs`). For traits that cannot use `automock`, use `mockall::mock!` instead.

### Test Naming Convention

```rust
// Pattern: test_<function>_<scenario>_<expected_outcome>
fn test_device_get_info_success() { }
fn test_device_get_info_unauthorized_returns_error() { }
fn test_media_create_profile_invalid_name_returns_validation_error() { }
```

## Streaming-Lib Testing

### Test Suites

| Suite | File | Purpose |
|-------|------|---------|
| FLV Muxing | `tests/flv_muxing_test.rs` | HTTP-FLV container format |
| HTTP-FLV Integration | `tests/httpflv_integration_test.rs` | HTTP-FLV serving |
| RTP Streaming | `tests/rtp_streaming_test.rs` | RTP packetization/depacketization |
| RTSP Session | `tests/rtsp_session_test.rs` | RTSP session lifecycle |
| RTSP Integration | `tests/rtsp_integration_test.rs` | RTSP server end-to-end |
| Stream Routing | `tests/stream_routing_test.rs` | Stream multiplexing |
| Streaming Service | `tests/streaming_service_test.rs` | Streaming service API |

```bash
cd cross-compile
$CARGO test --target x86_64-unknown-linux-gnu -p streaming-lib
```

## Validation Suite

**Location**: `validation/rust/`

Standalone validation tool for H.264 playback and RTSP RFC compliance testing against live cameras.

```bash
cd validation/rust
$CARGO test --target x86_64-unknown-linux-gnu
```

## WebUI Testing

### Test Types

| Type | Tool | Purpose |
|------|------|---------|
| Unit Tests | Vitest | Component logic, hooks, utils |
| Component Tests | Testing Library | UI rendering and interaction |
| Service Mocking | `vi.mock` | Mock service modules (`vi.mocked(fn).mockResolvedValue(...)`) |

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

### Service Mocking (vi.mock)

MSW is **not** used. Mock service modules with `vi.mock` + `vi.mocked`:

```typescript
import { getDeviceInfo } from '@/services/device';

vi.mock('@/services/device', () => ({
  getDeviceInfo: vi.fn(),
}));

// In a test
vi.mocked(getDeviceInfo).mockResolvedValue({
  manufacturer: 'Anyka',
  model: 'AK3918',
});
```

Shared helpers live in `src/test/componentTestHelpers.tsx`: `renderWithProviders`, `createTestQueryClient`, `mockToast`, `MOCK_ENDPOINTS`, `MOCK_DATA`, `waitForPageLoad`, `openDialog`, plus `formTestHelpers`, `dialogTestHelpers`, `mutationTestHelpers`, `serviceTestHelpers`, `schemaTestHelpers`, and `setup.ts`. Always render components with `renderWithProviders` so React Query providers are present.

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
| Rust | cargo-llvm-cov | Report to SonarCloud |
| WebUI | v8 coverage | Report to SonarCloud |

### Quality Gates

All tests must pass before merge:
```bash
# Rust (host target for testing)
$CARGO test --target x86_64-unknown-linux-gnu

# WebUI
npm run test
```
