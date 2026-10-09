# Traps from the `.127` / `.146` anyka-init cutover

Recorded 2026-10-09 from the retired `2026-08-11-anyka-init-cutover-127-146` plan, which had
no design doc. That file carried an `## Outcome` section, then three dated `Correction`
sections that overturned it. Only the corrected conclusions are here.

## Root causes that were misdiagnosed first

- **The `.146` streaming failure was a missing `users.toml`.** It was first blamed on a
  lock-ordering deadlock between the streaming and IPC paths. It was not a deadlock.
- **The `zt9101` path is not hardware-proven.** It hard-resets the camera. The cutover notes
  claimed it proven; that claim was retracted the same day.
- **The SD unmount does not kill the supervisor loop.** `.146` runs the same code with the
  same unmount and is fine.
- **The VI max-attr mapping is not a bug.** `video_input.rs:138-146` inverts `main_max` and
  `sub_max` on purpose, mirroring the libre_anyka_app workaround: in vendor IPC mode
  `main.max_*` drives sub-channel validation.

## Traps

- **Never `killall busybox` on the camera.** `udhcpc` and your own telnet shell are busybox.
- **`run_libre_anyka_app.sh` bricks a gergehack camera's video.**
  `SD_card_contents/anyka_hack/libre_anyka_app/run_libre_anyka_app.sh` hardcodes
  `image_width=1920 / image_height=1080`; upstream dropped the `gergesettings.txt` sourcing
  on the assumption that anyka-init cameras have no gergehack settings.
- **Log dry-run scripts to `/tmp`, never to the mount.** A failed `exec >/mnt/...` leaves
  busybox ash running, and a second `anyka-init` then fights the first over vendor-daemon's
  single-client IPC slots.
- **ONVIF is port 80, not 8080.**
- **Diff the deployed config against the on-device config before uploading.**
  `.deploy/anyka-127.toml` carried the wrong wifi PSK: the copy already on the camera matched
  `/data/gergesettings.txt`, the local one did not.

## Still open

Is there on-site physical access to `.127` or `.146`? The deadman in
`SD_card_contents/Factory/config.sh` covers wifi loss, but it is the last line of defence
and has never had to fire on a `zt9101` camera. If nobody can reach those cameras to pull an
SD card, a `zt9101` cutover is the riskiest thing in the fleet.
