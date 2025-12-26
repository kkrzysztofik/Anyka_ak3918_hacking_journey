# Troubleshooting

## ONVIF Server Not Responding

- Check if the `onvif-rust` process is running: `ps | grep onvif-rust`
- Verify the server port is not blocked: `netstat -ln | grep 8080`
- Check server logs for error messages
- Ensure the Rust binary is properly compiled and deployed

## PTZ Controls Not Working

- **Note**: Currently using stub implementation - hardware integration is not yet implemented
- Ensure PTZ hardware is properly initialized (when hardware integration is available)
- Check ONVIF server logs for errors
- Verify profile token is correct (default: "MainProfile")
- Ensure the platform abstraction layer is correctly configured

## Imaging Controls Not Working

- **Note**: Currently using stub implementation - hardware integration is not yet implemented
- Check if imaging service is enabled in the ONVIF server
- Verify video source token is correct
- Ensure camera supports imaging adjustments (when hardware integration is available)
- Check platform abstraction layer for hardware support

## See Also

- [[ONVIF-Rust-Implementation]] - ONVIF server implementation details
- [[Development-Environment]] - Toolchain and build setup
- [[Development-Guide]] - Development workflow and debugging
