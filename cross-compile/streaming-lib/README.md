# streaming-lib

Streaming library for RTSP and HTTP-FLV protocols, forked from [xiu](https://github.com/harlanc/xiu) for Anyka AK3918 hardware integration.

## Overview

This library provides minimal streaming components extracted from the xiu media server project. It is designed for use in the ONVIF Rust implementation on resource-constrained embedded systems (24MB memory budget).

## Components

- **RTSP Server**: ONVIF-compliant RTSP streaming server
- **HTTP-FLV Server**: Browser-compatible HTTP-FLV streaming with MSE support
- **H.264 Codec**: H.264 video codec parsing and handling
- **FLV Container**: FLV format muxing and demuxing
- **StreamHub**: Stream management and routing between protocols
- **BytesIO**: Binary I/O utilities for network operations
- **Common**: Shared utilities (auth, HTTP parsing, etc.)

## Modifications from xiu

- **Minimal extraction**: Only RTSP, HTTP-FLV, and required dependencies
- **ARMv5TEJ compatibility**: Patches applied for portable-atomic (no 64-bit atomics)
- **uClibc support**: Patches for openssl-src to support armv5te-unknown-linux-uclibceabi
- **Unified crate**: All components merged into a single library (not separate crates)
- **Anyka integration**: Prepared for integration with Anyka AK3918 hardware platform

## Usage

### RTSP Server

```rust
use streaming_lib::{RtspServer, StreamHubEventSender};

let event_sender: StreamHubEventSender = /* ... */;
let mut rtsp_server = RtspServer::new(
    "0.0.0.0:554".to_string(),
    event_sender,
    None, // auth
);
rtsp_server.run().await?;
```

### HTTP-FLV Server

```rust
use streaming_lib::HttpFlvServer;

let httpflv_server = HttpFlvServer::new(/* ... */);
// Start server
```

## Dependencies

All dependencies are specified in `Cargo.toml`. ARMv5TEJ compatibility patches are inherited from the workspace root (`cross-compile/Cargo.toml`).

## License

MIT License - see LICENSE file for details.

## Attribution

This project includes code from xiu (https://github.com/harlanc/xiu), Copyright (c) 2020 HarlanC, licensed under MIT License.

See NOTICE file for detailed attribution and modification notes.

## Building

This is a workspace member. Build from the workspace root:

```bash
cd cross-compile
cargo build --release
```

Or build just this library:

```bash
cd cross-compile/streaming-lib
cargo build --release
```

## Testing

```bash
cd cross-compile/streaming-lib
cargo test
```
