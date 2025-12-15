# Development Guide

## ONVIF Rust Development

For developing the ONVIF Rust implementation:

1. **Setup Environment**: Follow the [[Development-Environment]] guide
2. **Build and Test**: Use `cargo build` and `cargo test` with the custom toolchain
3. **Code Quality**: Run `cargo clippy` and `cargo fmt` for linting and formatting
4. **Add Services**: Implement new ONVIF services following the existing patterns

### Rust Development Commands

```bash
cd cross-compile/onvif-rust

# Use the custom toolchain's cargo
export CARGO=/home/kmk/anyka-dev/toolchain/arm-anykav200-crosstool-ng/bin/cargo

# Build for host (testing)
$CARGO build
$CARGO test

# Build for target (Anyka AK3918)
$CARGO build --release --target armv5te-unknown-linux-uclibceabi

# Code quality checks
$CARGO clippy -- -D warnings
$CARGO fmt --check
$CARGO fmt
```

## Web Interface Development

To modify the web interface:

1. **Edit CGI Scripts**: Modify the shell scripts in `www/cgi-bin/`
2. **Update SOAP Requests**: Modify the XML payloads for ONVIF requests (communicates with Rust ONVIF server)
3. **Add New Features**: Create new CGI scripts for additional ONVIF services
4. **Test Changes**: Use the startup script to test modifications

## Git Hooks

To install repository-provided git hooks that run local validations (e.g., Rust `cargo fmt` for `onvif-rust`), run:

```bash
scripts/install-git-hooks.sh
```

To revert this change run:

```bash
git config --unset core.hooksPath
```

## CI/CD Integration

The tools can be integrated into CI/CD pipelines:

```yaml
# For ONVIF Rust project
- name: Run Rust Linting
  run: |
    cd cross-compile/onvif-rust
    cargo clippy -- -D warnings
    cargo fmt --check
```

## Code Quality Standards

### Rust Project

- Use `cargo clippy -- -D warnings` for linting
- Use `cargo fmt --check` for formatting verification
- All code must pass tests: `cargo test`
- Follow Rust best practices and ownership model

### Static Analysis

For comprehensive code quality and security analysis, see [[Static-Analysis-Tools]].

## See Also

- [[Development-Environment]] - Toolchain setup and build instructions
- [[ONVIF-Rust-Implementation]] - ONVIF server implementation details
- [[Web-Interface]] - Web interface documentation
- [[Static-Analysis-Tools]] - Code quality and security analysis
