# Troubleshooting

## Camera Unreachable After SD Boot

- Confirm you edited `[wifi] ssid` / `password` in `anyka.toml` (shipped values are `CHANGE_ME`).
- Try recovery telnet: `telnet <camera-ip> 24`.
- On the camera, check `/mnt/logs/anyka-init.log` and that `/mnt/anyka_hack/anyka-init.bin` exists with interpreter `/mnt/anyka_hack/lib/ld-uClibc.so.1`.
- If the log mentions SAFE MODE, fix the fault, remove `/mnt/anyka_hack/state/boot.json`, reboot.
- See [[Boot-Runtime-Supervisor]].

## ONVIF Server Not Responding

- Confirm `anyka-init` started the service: `grep started /mnt/logs/anyka-init.log`
- Check if the `onvif-rust` process is running: `ps | grep onvif-rust`
- Verify the server port is not blocked: `netstat -ln | grep 8080`
- Check `/mnt/logs/onvif.log` and `/mnt/logs/anyka-init.log`
- A wrong system clock rejects authenticated ONVIF — confirm time sync in the supervisor log
- Ensure the Rust binary is properly compiled and deployed (`./scripts/build_sd_contents.sh`)

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

- [[Boot-Runtime-Supervisor]] - Init system, config, and SAFE MODE
- [[ONVIF-Rust-Implementation]] - ONVIF server implementation details
- [[Development-Environment]] - Toolchain and build setup
- [[Development-Guide]] - Development workflow and debugging
