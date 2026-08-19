# Network Pane — Write Support

**Date:** 2026-08-19
**Status:** Design approved — implemented in `docs/plans/2026-08-19-network-pane.md` and the corresponding anyka-init / onvif-rust / WebUI changes.

## Problem

The WebUI Network pane renders real read data but **every write path on it is dead**.
The page shows a Save button, a confirmation dialog, and a success toast for operations
that cannot succeed:

1. **`SetNetworkInterfaces` is not dispatched at all.** `networkService.setNetworkInterface()`
   (`cross-compile/www/src/services/networkService.ts:142`) sends the SOAP action, but
   `device/service.rs` has an arm for `GetNetworkInterfaces` (`:482`) and none for the
   setter. The request faults as an unknown action. DHCP/static IP cannot be changed.

2. **`SetDNS` returns `ActionNotSupported`.** `handle_set_dns`
   (`onvif-rust/src/onvif/device/ops/network.rs:333`) is a stub, wired into dispatch at
   `device/service.rs:513`. The DNS card's save silently fails.

3. **`SetNetworkProtocols` returns `ActionNotSupported`.**
   (`ops/network.rs:494`, dispatched at `device/service.rs:548`.) The Ports card never
   reads `GetNetworkProtocols` either — HTTP/HTTPS/RTSP are hardcoded to 80/443/554 in
   `NetworkPage.tsx:476-548` and never sent anywhere.

4. **The success toast fires regardless.** `mutation.onSuccess` (`NetworkPage.tsx:150`)
   reports "Network settings saved" for calls that returned SOAP faults, so the pane
   actively misreports its own state.

5. **The Hostname field is bound to the wrong data and duplicates another pane.**
   `NetworkPage.tsx:132` sets it from `iface?.name`, which is the *interface* name
   (`eth0`/`wlan0`), not a hostname. The real hostname editor shipped on
   IdentificationPage in `bb2a6e86`. Two panes writing one value is a latent
   last-writer-wins bug.

6. **The ONVIF Discovery switch is a pure stub** (`NetworkPage.tsx:318`), never sent.
   The real discovery-mode toggle also shipped on IdentificationPage.

7. **Gateway is hardcoded empty.** `networkService.ts:89` returns `gateway: ''` for every
   interface, even though `GetNetworkDefaultGateway` is implemented and working
   (`ops/network.rs:401`).

8. **HTTPS has no listener.** No TLS bind exists anywhere in `onvif-rust`; the HTTPS Port
   input can never be anything but decorative.

## Key discovery: the apply path already exists, in anyka-init

The network apply logic is **not** missing — it lives in the supervisor, not in
onvif-rust. `anyka.toml [wifi]` already carries `dhcp`, `address` (CIDR), `gateway`,
`dns[]` and `interface`, and `wifi.rs:661-686` already implements the full
static-vs-DHCP bring-up:

```rust
// R12: a typo'd static address associates fine and leaves the camera
// unreachable, which no rung of R7 would catch. Verify, then fall back to
// DHCP once before giving up.
if gateway_reachable(&gw) { return Ok(cidr.address); }
tracing::error!("static address assigned but gateway unreachable; retrying via DHCP");
dhcp_once(sys, cfg, layout)
```

The seams on the onvif-rust side exist too: `NetworkInfo` declares
`set_network_interface` (`traits.rs:766`), `set_dns` (`:779`) and `set_gateway` (`:788`)
as default methods returning `NotSupported`. The Anyka implementation simply never
overrides them.

**Consequence for the design:** onvif-rust must not run `ifconfig` itself. Doing so would
race the supervisor that owns the interface and re-create the .127 revert pattern.
Settings are persisted for anyka-init to apply at next boot, which reuses the existing,
tested bring-up path *and* inherits its gateway-unreachable rollback for free.

## Design

### Ownership model

Two files, two owners, exactly one writer each.

| File | Owner | Contents | Written by |
|---|---|---|---|
| `/mnt/anyka_hack/anyka.toml` `[wifi]` | Operator (SD card, hand-edited) | baseline `ssid`, `password`, `chip`, `gpio_polarity`, plus default `dhcp`/`address`/`gateway`/`dns` | nothing — never touched by software |
| `/mnt/anyka_hack/network.toml` | Machine | only the keys the user changed: `ssid`, `password`, `security`, `dhcp`, `address`, `gateway`, `dns` | onvif-rust, serde round-trip |
| `/mnt/anyka_hack/onvif/config.toml` | onvif-rust | `server.port`, `rtsp_port` | onvif-rust, existing `ConfigStorage::save` |

`anyka.toml` is comment-rich, uses `deny_unknown_fields`, and holds Wi-Fi credentials. A
serde round-trip from onvif-rust would strip every comment and force onvif-rust to
serialize credentials it has no business touching. Format-preserving edits would need
`toml_edit`, which is **not** in `Cargo.lock` (`toml 1.1.4` pulls `toml_parser`/
`toml_writer` only) — and a new dependency on the ARMv5 build is exactly the failure mode
that merges green and breaks `release.yml` afterwards.

A machine-owned overlay avoids all three problems: it has no comments to lose, no
operator intent to clobber, and a factory reset of networking is `rm network.toml`.

Ports deliberately stay out of the overlay. They are onvif-rust's own listeners;
anyka-init has no business knowing them.

anyka-init's `config.rs` gains one merge step: load `anyka.toml`, then overlay
`network.toml` if present. `WifiCfg` does not change shape — the overlay is an
`Option`-per-key struct merged onto it.

### Data flow

Reads (all already implemented, some unwired):

```
GetNetworkInterfaces ───┐
GetDNS ─────────────────┤→ platform/anyka/network_info.rs → live system state
GetNetworkDefaultGateway┤   (/proc, /etc/resolv.conf, ifconfig)
GetNetworkProtocols ────┘
GET /api/network        →  network.toml overlay = pending state
```

`GET /api/network` is the only new read. Live system state cannot express "saved but not
yet applied", so the WebUI reads the overlay to render pending badges and to populate the
SSID field.

Writes — three ONVIF ops for third-party interop, one REST endpoint for our own UI, all
funnelling to a single writer so they cannot diverge:

```
SetNetworkInterfaces ───┐
SetDNS ─────────────────┤→ NetworkInfo::set_* ──┐
SetNetworkDefaultGateway┘   (traits.rs:766-789)  ├→ network_overlay.rs
                                                 │   single writer,
PUT /api/network ────────────────────────────────┘   atomic temp+rename

SetNetworkProtocols ────→ config.toml server.port / rtsp_port
```

- `SetNetworkInterfaces` needs a new dispatch arm in `device/service.rs`.
- `SetDNS` and `SetNetworkProtocols` replace their `ActionNotSupported` bodies.
- `/api/network` mounts on the existing authenticated `/api` router
  (`server.rs:628-653`), alongside `/diagnostics`, `/logs`, `/update`.

Wi-Fi credentials go over REST only. ONVIF's route for them is
`Extension/Dot11Configuration` inside `SetNetworkInterfaces` — a large XML type surface
for a single form. Recorded as the upgrade path, not built.

### Safety ladder

Wi-Fi credentials are the one setting `gateway_reachable()` cannot rescue: no association
means there is no gateway to probe. A third rung is inserted into the middle of the
existing ladder, in the `Err(e)` arm of `bring_up_with` (`wifi.rs:433-452`):

```
1. overlay applied, static IP, gateway unreachable
   → udhcpc                                    [EXISTS, wifi.rs:679-686]

2. overlay fails to associate at all           [NEW]
   → mv network.toml network.toml.bad
   → retry try_bring_up_with(baseline)

3. baseline also fails
   → vendor wifi_manage.sh start               [EXISTS, wifi.rs:446]
```

Rung 2 reuses `try_bring_up_with` unchanged and mirrors the quarantine-and-revert
semantics already in `update.rs`: quarantine the new thing, boot the last known-good. The
`.bad` file is what the WebUI reads back to report that the settings failed.

Without rung 2, a typo'd SSID costs an SD-card pull.

### UI

```
Network
├─ Status ................ unchanged (real today)
├─ Wi-Fi Network ......... NEW    ssid / password / security
├─ IP Configuration ...... dhcp / address / prefix / gateway
│                          gateway now real (GetNetworkDefaultGateway)
├─ DNS ................... from-DHCP / primary / secondary
└─ Ports ................. HTTP / RTSP        (HTTPS input deleted)

REMOVED: hostname input, ONVIF discovery switch
         → link to Settings › Identification
```

Every card that writes the overlay carries a **pending badge**, driven by
`GET /api/network`, shown while the saved value differs from the live value. The confirm
dialog gains a "Reboot now" action: a save that requires a reboot the user never performs
is indistinguishable from no save at all.

### Error handling

- `ActionNotSupported` and every other SOAP fault must surface as a failure toast. The
  current `onSuccess` path (`NetworkPage.tsx:150`) reporting success for faulted calls is
  the defect that makes the pane untrustworthy today.
- The pending badge is the honest signal that nothing has changed *yet*.
- `network.toml.bad` present on boot renders a dismissible banner: "previous Wi-Fi
  settings failed; reverted to baseline."
- Overlay writes are atomic (temp + rename). A power cut mid-write must leave either the
  old overlay or no overlay, never a half-parsed one — a corrupt overlay that fails
  `deny_unknown_fields` would take rung 2 on every boot.

### Testing

| Layer | Test |
|---|---|
| anyka-init | overlay merge precedence; rung-2 quarantine-and-retry against the existing tempdir `FsLayout` harness |
| onvif-rust | overlay writer round-trip; `SetNetworkInterfaces` / `SetDNS` / `SetNetworkProtocols` handlers via `MockNetworkInfo` |
| WebUI | pending badge, `.bad` banner, ports confirm dialog, removal of the duplicated fields |
| Hardware | static IP with wrong gateway → DHCP rescue; wrong SSID → baseline rescue |

Hardware validation runs on **.198** (telnet reachable, per `anyka-device-access`), not
.127, which is on the legacy stack and reverts.

## Assumptions

Two points were defaulted rather than explicitly confirmed; either can be flipped.

1. **Rung 2 is in scope**, i.e. this branch modifies anyka-init. The rescue is what makes
   exposing Wi-Fi credentials safe, so shipping credentials without it is not proposed.
   The alternative is to split: WebUI + onvif-rust first, overlay + rescue second.
2. **Ports are writable**, with the resulting URL spelled out in the confirm dialog. A
   port change takes effect only at reboot and looks identical to a hang until the user
   reconnects on the new port. The alternative is read-only ports, leaving
   `SetNetworkProtocols` as `ActionNotSupported`.

## Out of scope

- `SetNTP` (`ops/network.rs:395`) is also `ActionNotSupported`, but NTP belongs to
  TimePage (`TimePage.tsx:123`), not this pane.
- IPv6. No part of the current stack reads or writes it.
- HTTPS/TLS listener.
- ONVIF `Dot11Configuration` types for Wi-Fi over SOAP.
