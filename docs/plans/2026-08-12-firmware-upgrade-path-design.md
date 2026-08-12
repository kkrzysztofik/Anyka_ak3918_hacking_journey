# Firmware Upgrade Path — Design

Date: 2026-08-12
Scope: `anyka-init`, `vendor-daemon`, `onvif-rust` on the AK3918 fleet.

## Problem

Updating a camera today means an FTP push of one binary at a time
(`scripts/deploy_onvif.sh`), a `killall` over telnet to dodge `ETXTBSY`, and a
size comparison to prove the upload landed. There is no version identity on the
device, no rollback beyond ad-hoc `.bak` files, and nothing stops the three
components from drifting out of sync. Two of the four cameras are reachable only
through `root@192.168.3.137`, so a bad push costs a site visit.

Four goals, all in scope:

1. Atomic apply with automatic rollback.
2. One command per camera, jumphost included.
3. Version identity visible from the WebUI/ONVIF.
4. One versioned bundle rather than per-binary pushes.

## Decisions

| Decision | Choice |
|---|---|
| Activation | A/B slots, `active` pointer file, **always reboot** |
| Applier location | `anyka-init`, driven by a spool directory |
| Trial predicate | Listening sockets from `/proc/net/tcp` |
| Trial state | Marker file, no parsing |
| Bundle | Three components + per-file sha256 manifest, no signing |
| Device config | Lives outside the slots, never overwritten |
| Transport | Any drop into `spool/`; `PUT /api/update` in increment 2 |

The manifest is `manifest.sha256` in `sha256sum -c` format plus a
`manifest.meta` of `key=value` lines, not JSON: `anyka-init` has no
`serde_json`, and `sha256sum -c` does the whole verify in one exit status using
a busybox applet already on the device. See the implementation plan's
"Deviation from the design doc" for the full reasoning.

`lib/` (31 MB of uClibc runtime) stays outside the slots and outside the bundle.
A toolchain bump becomes a deliberate separate step; a mismatch surfaces as a
failed trial rather than a silent bad flip.

## On-disk layout

```
/mnt/anyka_hack/
  active                     # text: "a" or "b" — read by config.sh and anyka-init
  slots/
    a/
      manifest.sha256        # sha256sum -c format, one line per file
      manifest.meta          # version=, requires_config_schema=
      anyka-init.bin
      vendor-daemon/{vendor-daemon.bin, lib/}
      onvif/{onvif-rust.bin, www/, config.template.toml}
    b/                       # same shape
  anyka.toml                 # device-local. never in a slot
  onvif/config.toml          # device-local
  lib/                       # shared uClibc runtime, outside slots
  state/
    boot.json                # existing storm guard
    trial-<slot>             # exists ⇒ unconfirmed update; name carries prev slot
  spool/
    bundle.tar
    bundle.trigger           # written last; existence means "complete, go"
```

### `config.sh`

`SD_card_contents/Factory/config.sh:18` hardcodes the supervisor path. It becomes
slot-aware, and gains a fallback to the other slot:

```sh
SLOT=$(cat /mnt/anyka_hack/active 2>/dev/null) || SLOT=a
BIN=${ANYKA_INIT_BIN:-/mnt/anyka_hack/slots/$SLOT/anyka-init.bin}
[ -x "$BIN" ] || BIN=/mnt/anyka_hack/slots/$([ "$SLOT" = a ] && echo b || echo a)/anyka-init.bin
```

Selected slot unusable → other slot → existing `exit 1` at `:59` → the existing
240-second deadman at `:45-57` restores the vendor boot path and reboots. Three
tiers, the outer two already proven on hardware.

### Slot-relative paths

`sys.rs:108-110` builds children with `Command::new(&spec.exec).env_clear()` and
no working directory, so `onvif-rust`'s `static_root = "www"`
(`config_debug.toml:129`) resolves against `CWD=/` — the bug that killed the
WebUI once already. One line fixes it and makes paths slot-relative for free:

```rust
cmd.current_dir(spec.exec.parent().unwrap_or(Path::new("/")));
```

No `onvif-rust` change, no TOML templating on flip. Caveat: services with
`core_dump = true` write cores to CWD, so this moves them from `/` onto the SD
card. Verify that is acceptable before merging.

## Apply flow

Polled from the existing supervisor tick (`[monitor] interval_sec = 60`); a
`stat` of `spool/bundle.trigger` costs nothing.

```
tick:  spool/bundle.trigger present?
         → untar spool/bundle.tar into slots/<inactive>
         → sha256sum -c manifest.sha256, run with CWD = the slot
         → compare manifest.meta requires_config_schema against anyka.toml schema
         → any failure: log, clear spool, wipe staging, done (nothing flipped)
         → touch state/trial-<current>, sync
         → write active = <inactive>, sync
         → reboot

boot:  state/trial-<prev> present?
         no  → normal start
         yes → poll /proc/net/tcp for LISTEN on 80, 554, 8080
                 all three, sustained 30 s, deadline 120 s
                   → unlink state/trial-<prev>          (committed)
                 else
                   → write active = <prev>, sync, reboot
```

Because new files land in the *inactive* slot, a running binary is never
overwritten. `ETXTBSY` becomes structurally impossible, which retires the
size-check workaround at `deploy_onvif.sh:132-160` and the `killall` step it
documents. Post-write sha256 also subsumes the hand-built md5 manifest the
`tar|nc` workflow needs to catch exFAT NUL-byte writes.

### Trial predicate

`anyka-init` depends on `serde, toml, tracing, libc, signal-hook, thiserror,
anyhow` — no HTTP client, no `serde_json`. Polling `/api/diagnostics` would add
an HTTP stack and a JSON parser to a 1.5 MB supervisor for one GET.

`netstat.rs` already parses `/proc/net/route` and `/proc/net/arp`. Add
`/proc/net/tcp` in the same idiom:

```rust
pub fn listening(port: u16) -> bool {
    std::fs::read_to_string("/proc/net/tcp").is_ok_and(|s| {
        s.lines().any(|l| {
            let mut f = l.split_whitespace().skip(1);
            f.next().and_then(|a| a.rsplit(':').next().and_then(|p| u16::from_str_radix(p, 16).ok()))
                == Some(port)
                && f.next().is_some_and(|st| st == "0A") // TCP_LISTEN
        })
    })
}
```

Ports 80 (ONVIF/HTTP), 554 (RTSP) and 8080 (HTTP-FLV) from
`SD_card_contents/anyka_hack/onvif/config.toml:105,183,186`.

This is a better signal than the health JSON for the failure mode that actually
occurs here: HTTP serving fine while startup never completes and RTSP/FLV never
bind is exactly a bound-80/unbound-554 state.

Known ceiling: bound sockets do not prove frames flow, so a broken
`vendor-daemon` can pass the trial. Mark it in code:
`// ponytail: socket-liveness smoke test, add a frame-counter probe if a silent-no-video regression ships.`

### Trial state

No `serde_json`, and the storm guard already established that parsing state off
exFAT after a power cut is a hazard (`boot-runtime-rust-design.md:449`). So parse
nothing: `state/trial-a` existing means an unconfirmed update whose previous slot
was `a`. Existence is the flag, the filename is the payload, `unlink` is the
commit. No torn-write failure mode.

### Config schema check

`manifest.meta` declares `requires_config_schema=N`; `anyka.toml` gains
`schema = N`. The *old* applier compares two integers before flipping, so a build
needing new config keys fails at verify time with both slots intact rather than
crashlooping into a revert.

Requires adding `schema` to the template
(`SD_card_contents/anyka_hack/anyka.toml`) and all four device configs
(`.deploy/anyka{,-121,-127,-146}.toml`).

## Failure matrix

| Failure | Caught by | Result |
|---|---|---|
| Truncated or corrupt transfer | sha256 vs manifest, pre-flip | Both slots intact |
| exFAT NUL-byte write artifact | same, post-write | Both slots intact |
| New build needs config keys the device lacks | schema check, pre-flip | Both slots intact |
| New build crashloops | trial: ports never bind | `active` = prev, reboot |
| New build up but RTSP never binds | trial: 554 unbound | `active` = prev, reboot |
| New `anyka-init` will not exec | `config.sh` slot fallback | Boots previous slot |
| Both slots dead | `config.sh` deadman (`:45-57`) | Vendor boot path, reboot |

## Version identity

`git describe` at build time, compiled into `onvif-rust` via
`option_env!("ANYKA_BUILD_VERSION")` and written into `manifest.meta` from the
same source. Surfaced in ONVIF `GetDeviceInformation`'s `FirmwareVersion` and in
`/api/diagnostics`. Not baked into `anyka-init` — nothing asks it.

## Transport

The applier works with any transport that can drop a file into `spool/`.

```bash
# increment 2, LAN. /api/update sits behind AuthLevel::Administrator, so the
# admin credentials from onvif config.toml are required:
curl -u admin:PASSWORD -T bundle.tar http://192.168.2.198/api/update

# increment 2, behind the jumphost
ssh root@192.168.3.137 'curl -u admin:PASSWORD -T - http://192.168.30.x/api/update' < bundle.tar
```

`PUT /api/update` streams the body straight to `spool/bundle.tar` — a raw body,
not multipart, so there is no parser and nothing buffers 19 MB in 36 MB of RAM —
then writes `spool/bundle.trigger` on clean EOF and returns 202. Reuses the
existing `AuthLevel::Administrator` gate (`diagnostics/http.rs:89`).

FTP and telnet remain the recovery path for a camera whose `onvif-rust` is dead.

Bundle building belongs beside `scripts/build_sd_contents.sh`. There is no deploy
wrapper script; the deploy command is `curl -u admin:PASSWORD -T`.

## Increments

1. **Applier.** Slots, `active`, `config.sh` fallback, trial, revert, schema
   check, `current_dir`. Transport is FTP into `spool/`, which works today.
   **Zero `onvif-rust` changes.** Proves rollback in isolation.
2. **Transport.** `PUT /api/update`, WebUI upload button, `FirmwareVersion`.

## Testing

Host-side, no hardware:

- `tests/p0_wrapper.rs` already stubs `config.sh` through `ANYKA_INIT_BIN`; the
  slot fallback and its two-tier degradation test there.
- One new integration test in `anyka-init`: apply a synthetic bundle into a
  tempdir, assert the flip and the marker file; a second case asserting revert
  when the trial predicate never passes.
- `netstat::listening` unit-tests against a fixture `/proc/net/tcp`.

## Rejected

- **Hot-swap without reboot.** Saves ~90 s per update, costs a second code path
  with its own trial-resumption story and mixed-slot states.
- **`lib/` hash pinning** and a **six-phase persisted state machine.** Both
  predict failures the trial already catches; they buy a nicer error message.
- **Applier in `onvif-rust`.** Cannot restart itself or replace `anyka-init`, so
  it needs supervisor IPC anyway — strictly more work for less safety.
- **Shell applier.** Rollback logic in shell, on the path where a mistake costs a
  site visit. `config.sh` earns its shell status by depending on nothing; an
  applier has no such excuse.
- **Signing.** Both entry points already require admin auth, and admin on this
  camera already implies telnet.
