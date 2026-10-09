# Vendor wifi scripts: what the Rust rewrite had to work around

Recorded 2026-10-09 from the retired `2026-08-01-wifi-bring-up` and
`2026-08-01-wifi-findings-remediation` plans, which had no design doc of their own. The
vendor sources are under `SD_card_contents/anyka_hack/orig/data/`.

## Defects in the vendor scripts

**W1 — function names built from strings silently lose entries.**
`wifi_driver.sh:386` dispatches with `wifi_config_${WIFI_NAME} 1`. Two of the ten network
entries have no matching function, so they resolve to "command not found" with no
diagnostic. The Rust rewrite uses an exhaustive `match` so a missing arm is a compile error.

**W3 — credentials are interpolated into a shell string.**
`station_connect.sh:89-91` interpolates SSID and PSK into `sh -c`. A `"`, `$`, backtick or
`\` breaks the quoting. This is the cause of the vendor claim that "some special
characters don't work in wifi ssid names and passwords" — it is a quoting bug, not a
limitation, and the fix is to stop passing credentials through a shell rather than to
reject characters.

**W6 — wifi is the camera's only recovery channel.**
Every wifi change is also a change to the only path back to the device. This is why the
bring-up concentrated its risk in Phase A and why the deadman in
`SD_card_contents/Factory/config.sh` exists.

## Findings from the first hardware validation

F1–F4 were blocking and are fixed; they are recorded here because each one reads as
success rather than failure.

- **F1 — a failed DHCP read as success.** `read_address` returned `0.0.0.0` when `udhcpc`
  got no lease, so the monitor reported connected.
- **F2 + F3 — two `wpa_supplicant` processes, and the probed driver flag discarded.**
  Same handover gap, fixed together.
- **F4 — the shipped config contradicted itself.** `SD_card_contents/anyka_hack/anyka.toml:27`
  disagreed with `:69-73`.

**F5 is deliberately still there.** `gateway_reachable` in
`cross-compile/anyka-init/src/netstat.rs` calls `std::thread::sleep(200ms)` directly,
blocking the monitor thread on every tick. It matches the source plan, so it is not a
regression; it carries a `ponytail:` comment naming the ceiling.
