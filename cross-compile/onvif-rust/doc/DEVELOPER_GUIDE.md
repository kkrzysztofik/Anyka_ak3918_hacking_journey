# Developer Guide: Anyka AK3918 ONVIF Services (Rust)

This guide provides step-by-step instructions for setting up the development environment, building, testing, and deploying the Rust-based ONVIF stack for the Anyka AK3918 camera.

## 1. Prerequisites

Ensure you have the following installed on your development machine (Linux/WSL2 recommended):

- **Rust Toolchain**: Stable channel (Edition 2024).

  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

- **Cross-Compilation Toolchain**: `arm-anykav200-crosstool-ng`.
  - Ensure the toolchain binaries (e.g., `arm-anykav200-linux-uclibcgnueabi-gcc`) are in your `PATH`.
- **Git**: For version control.

## 2. Environment Setup

### 2.1. Configure Rust for Cross-Compilation

The Anyka AK3918 uses an ARM926EJ-S processor (ARMv5TE) and uClibc-ng.

1. **Add the ARMv5TE Target**:

    ```bash
    rustup target add armv5te-unknown-linux-uclibceabi
    ```

    *Note: If the strict uClibc target is not available via rustup, you may use `armv5te-unknown-linux-gnueabi` with a custom linker configuration, or use `cross`.*

2. **Linker Configuration**:
    Ensure your `.cargo/config.toml` is configured to use the Anyka toolchain linker:

    ```toml
    [target.armv5te-unknown-linux-uclibceabi]
    linker = "arm-anykav200-linux-uclibcgnueabi-gcc"
    ```

## 3. Building the Project

### 3.1. Host Build (Development)

For rapid logic testing (not hardware dependent), you can build for your host machine:

```bash
cargo build
```

### 3.2. Target Build (Production)

To build the binary for the camera:

```bash
cargo build --release --target armv5te-unknown-linux-uclibceabi
```

The resulting binary will be located at:
`target/armv5te-unknown-linux-uclibceabi/release/onvif-rust`

## 4. Testing

### 4.1. Unit Tests

Run standard unit tests on your host machine:

```bash
cargo test
```

This runs all tests in `src/` and `tests/`. We use `mockall` and `wiremock` to simulate hardware and network interactions.

### 4.2. Linting & Formatting

Adhere to the project's quality standards:

```bash
cargo fmt --check
cargo clippy -- -D warnings
```

## 5. Adding a New ONVIF Service

Follow this process to add a new service (e.g., `Events`):

1. **Define Types**:
    Create `src/onvif/types/events.rs`. Define the request/response structs using `serde` and `quick-xml`.

    ```rust
    #[derive(Debug, Deserialize)]
    pub struct CreatePullPointSubscription { ... }
    ```

2. **Implement Logic**:
    Create `src/onvif/events/mod.rs`. Implement the service logic.

    ```rust
    pub async fn handle_create_pull_point(...) -> Result<Response, OnvifError> { ... }
    ```

3. **Register Route**:
    Update `src/onvif/dispatcher.rs` to route requests to your new handler based on the SOAP action or path.

4. **Update WSDL**:
    Ensure the `GetServices` response in `src/onvif/device/handlers.rs` includes the new service capability.

## 6. Deployment (Hardware)

To run the new firmware on the Anyka AK3918:

1. **Build Release Binary**:

    ```bash
    cargo build --release --target armv5te-unknown-linux-uclibceabi
    ```

2. **Prepare SD Card**:
    - Mount your SD card.
    - Copy the binary to the hack directory:

      ```bash
      cp target/armv5te-unknown-linux-uclibceabi/release/onvif-rust /path/to/sdcard/anyka_hack/bin/
      ```

    - Ensure the startup script (`anyka_hack/hack.sh` or similar) executes `onvif-rust`.

3. **Boot**:
    - Insert SD card into the camera.
    - Power cycle.
    - Check logs via UART or `http://<camera-ip>:8080/log` (if configured).

## 7. Debugging

- **Logs**: Use `RUST_LOG` environment variable to control verbosity.

  ```bash
  RUST_LOG=debug ./onvif-rust
  ```

- **Backtraces**: Enable backtraces for crashes.

  ```bash
  RUST_BACKTRACE=1 ./onvif-rust
  ```
