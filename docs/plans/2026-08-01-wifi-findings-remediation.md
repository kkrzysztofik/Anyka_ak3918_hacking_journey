# Wifi Bring-Up — Remediation of Validation Findings F1–F7

> **Handover prompt.** You are picking up finished work that passed its own
> tests but has seven defects found in review. Everything you need is in this
> file. Do not re-derive the plan; do not re-audit the vendor scripts.

**Origin:** validation of `docs/plans/2026-08-01-wifi-bring-up.md` after Tasks
A1–A9 and B1–B6 were implemented and merged on branch
`design/boot-runtime-rust` (commits `ebbf98fd`..`f9eb62d3`).

**State on arrival:** 109 lib tests pass, 4 integration tests pass, clippy clean,
`fmt --check` clean, cross-compile succeeds. None of that catches the bugs
below — every defect is in impure glue or in shipped config, not in the tested
pure functions.

**Your job:** fix F1–F7 in order. F1–F4 are blocking; F5–F7 are cleanup.
Do not start hardware testing (`A10`/`B7` in the source plan) until F1–F4 land.

---

## Before You Start

### Toolchain — mandatory

From the repo root:

```bash
source ./setenv.sh
```

Exports `$CARGO` and prepends the vendored toolchain `bin/` to `PATH`. The
`PATH` prefix is **not** optional — `cargo clippy` dies with `E0514` without it.

### Commands

```bash
cd cross-compile/anyka-init

$CARGO test --target x86_64-unknown-linux-gnu --lib
$CARGO test --target x86_64-unknown-linux-gnu --lib test_parse_local     # one group
$CARGO test --target x86_64-unknown-linux-gnu --test supervision -- --test-threads=1
$CARGO clippy --target x86_64-unknown-linux-gnu --all-targets -- -D warnings
$CARGO fmt --check
```

Integration tests **must** be `--test-threads=1`; they call `waitpid(-1)`.

Cross-compile check, from the repo root:

```bash
bash ./scripts/build_sd_contents.sh
```

### Rules you must follow

- **TDD.** Write the failing test first, run it, confirm it fails for the right
  reason, then implement. This is how every existing module in this crate was
  built.
- Test names are `test_<subject>_<behaviour>`. Never `test1`, never `test_fix`.
- No `unwrap()` or `expect()` outside `#[cfg(test)]`. Use `Result` and `?`.
- Pure functions live next to their impure caller with a `#[cfg(test)] mod tests`
  at the bottom of the file.
- `#[serde(deny_unknown_fields)]` on every config struct.
- Comments explain *why*, and cite vendor source as `file:line` when the
  behaviour was transcribed from it.
- One commit per finding. Do not batch.

### Rules you must not break

- **Never disable, skip, or weaken an existing test to make your change pass.**
  If an existing test now fails, that is information — read it.
- Do not change `Chip::ALL`, `parse_hw_conf`, `validate_credentials`,
  `wpa_supplicant_conf`, `parse_cidr`, `resolv_conf`, `parse_default_route`,
  `arp_entry_complete`, or `decide`. All eight were verified correct against
  `orig/data/wifi_driver.sh:240-370` and are out of scope.
- Do not add dependencies. Everything below is `std` plus what is already in
  `Cargo.toml`.
- Do not refactor beyond the finding you are on.

---

## F1 — BLOCKING: `read_address` returns `0.0.0.0`, so a failed DHCP reads as success

**File:** `cross-compile/anyka-init/src/wifi.rs:594-618` (the function), called
only from `dhcp_once` at `:583-587`.

### What is wrong

```rust
fn read_address(iface: &str) -> Option<String> {
    let _ = iface;                                    // <-- argument discarded
    let src = std::fs::read_to_string("/proc/net/fib_trie").ok()?;
    ...
        if addr.contains('.') && addr != "127.0.0.1" {
            return Some(addr.to_string());            // <-- first match wins
        }
```

The `Local:` section of `/proc/net/fib_trie` begins with the `0.0.0.0/0` stub,
so the first `|-- ` line that satisfies that predicate is `0.0.0.0`.

### Proof

Transcribing the function to Python and running it against a real
`/proc/net/fib_trie` returns `'0.0.0.0'`. Reproduce it yourself before you fix
anything — you need to see the failure:

```bash
cat > /tmp/ra.py <<'EOF'
src = open('/proc/net/fib_trie').read()
in_local = False
for line in src.splitlines():
    if 'Local' in line:
        in_local = True; continue
    if not in_local: continue
    t = line.strip()
    if not t.startswith('|-- '): continue
    addr = t[4:].split()[0]
    if '.' in addr and addr != '127.0.0.1':
        print('read_address() returns:', repr(addr)); break
EOF
uv run python3 /tmp/ra.py && rm /tmp/ra.py
```

### Why it is blocking

```
udhcpc gets no lease
  -> read_address returns Some("0.0.0.0")
    -> dhcp_once returns Ok
      -> try_bring_up returns Outcome::Up { addr: "0.0.0.0" }
        -> fall_back() is never called      (the R7 vendor fallback is defeated)
        -> storm.wifi_reboots is zeroed     (the B4 reboot budget is wiped)
```

Wifi is the camera's only remote-recovery channel. A bring-up that reports
success while the camera has no address is exactly the failure class the whole
design exists to prevent. It also silently defeats the static-address
verification retry at `wifi.rs:573-580`, because that retry calls `dhcp_once`
and therefore always "succeeds".

### Required fix

Move the parse into `src/netstat.rs` as a **pure function** — that is where
every other `/proc` parser in this crate lives, and it is the reason they are
all correct and this one is not.

```rust
/// Host IPv4 for `iface`.
///
/// `/proc/net/fib_trie` carries the host addresses but no interface, and
/// `/proc/net/route` carries the interface but only network addresses. Joining
/// them gives an interface-attributed host address without an ioctl.
///
/// Both arguments are file *contents*, never paths, so this is provable on the
/// host.
pub fn parse_local_ipv4(fib_trie: &str, route: &str, iface: &str) -> Option<String>
```

Algorithm:

1. From `route`, collect every non-default row whose `Iface` field is `iface`
   (skip the header line, skip rows where `Destination == "00000000"`). Decode
   `Destination` and `Mask` — both little-endian hex, same as
   `parse_default_route` at `netstat.rs:21-38`, so reuse that decoding style.
   Result: a list of `(network: u32, mask: u32)`.
2. From `fib_trie`, walk the `Local:` section. A host address is a line
   `|-- <ip>` whose **next** line is exactly `/32 host LOCAL` after trimming.
   This is what excludes the three false positives that broke the old code:
   - `0.0.0.0` is followed by `/0 universe UNICAST`
   - `127.0.0.0` is followed by `/8 host LOCAL` (note: `/8`, not `/32`)
   - broadcast addresses are followed by `/32 link BROADCAST`
3. Return the first host address `a` such that `a & mask == network` for one of
   the subnets from step 1. Skip anything in `127.0.0.0/8`.
4. `None` if nothing matches.

Do **not** fall back to "return any host address" when the subnet join finds
nothing. Returning `None` is the correct answer — it is what makes `dhcp_once`
fail and the vendor fallback fire, which is the entire point of this fix.

The interface filter is load-bearing, not decoration:
`orig/usr/sbin/standby.sh:18` runs `ifconfig eth0 down`, so an `eth0` exists on
at least some board revisions of this hardware.

### Tests to write first

In `netstat.rs`'s existing `mod tests`. Use these fixtures — they are the real
kernel format, tabs in `route`, leading spaces in `fib_trie`:

```rust
const FIB_TRIE: &str = "\
Main:
  +-- 0.0.0.0/0 3 0 5
     |-- 0.0.0.0
        /0 universe UNICAST
Local:
  +-- 0.0.0.0/0 3 0 4
     |-- 0.0.0.0
        /0 universe UNICAST
     +-- 127.0.0.0/8 2 0 2
        +-- 127.0.0.0/31 1 0 0
           |-- 127.0.0.0
              /8 host LOCAL
           |-- 127.0.0.1
              /32 host LOCAL
        |-- 127.255.255.255
           /32 link BROADCAST
     +-- 192.168.2.0/24 2 0 2
        |-- 192.168.2.0
           /32 link BROADCAST
        |-- 192.168.2.198
           /32 host LOCAL
        |-- 192.168.2.255
           /32 link BROADCAST
";
```

Reuse the existing `ROUTE` constant already defined at `netstat.rs:143-147`
(`wlan0`, default route plus the `192.168.2.0/24` subnet row).

Required cases:

| Test name | Assertion |
|---|---|
| `test_parse_local_ipv4_returns_the_host_address_for_the_interface` | `Some("192.168.2.198")` for `wlan0` |
| `test_parse_local_ipv4_never_returns_the_default_route_stub` | result `!= Some("0.0.0.0")` — this is the regression guard for F1, name it so the next reader knows why it exists |
| `test_parse_local_ipv4_skips_loopback_and_broadcast` | never `127.0.0.0`, `127.0.0.1`, `192.168.2.0`, `192.168.2.255` |
| `test_parse_local_ipv4_none_for_other_interface` | `None` for `eth0` |
| `test_parse_local_ipv4_none_when_no_address_assigned` | `None` when `fib_trie` has the `Local:` header and loopback only |
| `test_parse_local_ipv4_handles_empty_and_malformed` | `None` for `("", "", "wlan0")` and for a `fib_trie` truncated mid-entry |

**Red-green discipline:** after all six pass, temporarily revert your
`parse_local_ipv4` body to the old "first `|--` that isn't 127.0.0.1" logic and
confirm `test_parse_local_ipv4_never_returns_the_default_route_stub` **fails**.
Restore, confirm green. A regression test that has never been seen red is not a
regression test.

### Then rewire the caller

In `wifi.rs`, delete `read_address` entirely and change `dhcp_once` to read both
files and delegate:

```rust
fn dhcp_once(sys: &dyn Sys, cfg: &WifiCfg) -> Result<String, String> {
    sys.run_to_completion(BUSYBOX, &udhcpc_oneshot_args(&cfg.interface))
        .map_err(|e| format!("udhcpc: {e}"))?;
    let fib = std::fs::read_to_string("/proc/net/fib_trie").unwrap_or_default();
    let route = std::fs::read_to_string("/proc/net/route").unwrap_or_default();
    crate::netstat::parse_local_ipv4(&fib, &route, &cfg.interface)
        .ok_or_else(|| format!("no address on {} after udhcpc", cfg.interface))
}
```

### Pass criteria

- Six new tests pass; the regression guard has been seen red.
- `read_address` no longer exists anywhere in the crate
  (`rg 'fn read_address' cross-compile/anyka-init/src` returns nothing).
- Full lib suite green, clippy clean, fmt clean.

### Commit

```
fix(anyka-init): interface-attributed local address, so a failed DHCP is a failure

read_address returned the 0.0.0.0 stub from /proc/net/fib_trie's Local section
regardless of interface, so dhcp_once reported success with no lease. That
suppressed the vendor fallback and wiped the wifi reboot budget on every boot.
```

---

## F2 + F3 — BLOCKING: duplicate `wpa_supplicant`, and the probed driver flag is discarded

Fix these together; they are the same handover gap and touch the same lines.

**Files:**
- `cross-compile/anyka-init/src/wifi.rs:313-324` (`Outcome`), `:389` (`try_bring_up`), `:462-463`, `:524-542`
- `cross-compile/anyka-init/src/boot.rs:104-118`
- `cross-compile/anyka-init/src/main.rs:54` and `:111`
- `SD_card_contents/anyka_hack/anyka.toml:75-79`

### What is wrong

**F2.** `start_supplicant_probing_driver` spawns a detached `wpa_supplicant`
(`wifi.rs:531-535`) and nothing ever stops it. Sequencing in `main.rs`:

```
main.rs:54   P2   boot::system_setup  ->  wifi::bring_up  ->  detached supplicant, still running
main.rs:57   P2.5 timesync::first_sync                        (needs the link up — do not break this)
main.rs:111  P3   supervisor_loop::run -> spawns [services.wpa_supplicant]
```

Two supplicants then share `ctrl_interface=/var/run/wpa_supplicant`. The
supervised one loses the control-socket bind and exits, the supervisor restarts
it under backoff, the crash-loop counter climbs, and the storm guard can take
the camera to safe mode or reboot.

**F3.** `start_supplicant_probing_driver` returns whichever of `nl80211` /
`wext` actually associated, but `try_bring_up:462-463` only logs it:

```rust
let driver = start_supplicant_probing_driver(sys, cfg)?;
tracing::info!(driver, "wpa_supplicant associated");   // value dropped here
```

`anyka.toml:78` hardcodes `-D nl80211` for the supervised service. If `wext` was
the flag that worked, the long-lived supplicant runs with the one that just
failed. That defeats the driver probe entirely.

### Required fix

**1. Carry the driver out of bring-up.** Add a field to the success variant:

```rust
pub enum Outcome {
    Up {
        chip: &'static str,
        ssid: String,
        addr: String,
        /// The `-D` flag that actually associated. The supervised service in P3
        /// must be started with this one, not with the config's default (R8).
        driver: &'static str,
    },
    FellBack,
    Failed,
}
```

Populate it at `try_bring_up`'s `Ok(Outcome::Up { .. })`. Update the match arms
in `boot.rs:104-118`.

**2. Return it from `system_setup`.** Change `boot::system_setup` to return
`Option<&'static str>` — `Some(driver)` on `Outcome::Up`, `None` otherwise.
Keep all existing logging.

**3. Patch the service spec and hand over, in `main.rs`.**

- Change `let cfg` at `main.rs:16` to `let mut cfg`.
- Capture the return: `let probed = boot::system_setup(sysimpl.as_ref(), &cfg);`
  — note you will need to end the immutable borrow before mutating; bind
  `probed` first, then mutate.
- After `timesync::first_sync` and **immediately before**
  `supervisor_loop::run(...)` at `main.rs:111`:

```rust
    // The supplicant bring-up spawned is unsupervised and holds the ctrl
    // socket. Hand the interface over to the supervised instance here, not
    // earlier: P2.5 time sync needs the link up, and the address stays
    // configured across the swap so the gap is one reassociation.
    if let Some(driver) = probed {
        if let Some(svc) = cfg.services.get_mut("wpa_supplicant") {
            if let Some(i) = svc.args.iter().position(|a| a == "-D") {
                if let Some(slot) = svc.args.get_mut(i + 1) {
                    slot.clear();
                    slot.push_str(driver);
                }
            }
        }
        let _ = sysimpl.run_to_completion("killall", &["wpa_supplicant".to_string()]);
    }
```

`cfg.services` is a `BTreeMap<String, ServiceCfg>` (`config.rs:45`) and
`ServiceCfg::args` is a `Vec<String>`, so `get_mut` works directly.

Ordering matters and is the one thing to get right here: killing earlier drops
the link under time sync; killing later means two supplicants coexist.

### Tests to write first

`supplicant_args` is already tested. What is untested is the arg patching, so
extract it as a pure function in `wifi.rs` and test it:

```rust
/// Rewrite the `-D <driver>` pair in a service argv. Returns false when the
/// service does not take a `-D` flag, which the caller logs.
pub fn patch_driver_arg(args: &mut [String], driver: &str) -> bool
```

| Test name | Assertion |
|---|---|
| `test_patch_driver_arg_replaces_the_flag_value` | `["-i","wlan0","-D","nl80211","-c","x"]` + `"wext"` → `-D` followed by `wext` |
| `test_patch_driver_arg_leaves_other_args_untouched` | length and every other element unchanged |
| `test_patch_driver_arg_returns_false_without_a_d_flag` | `["-i","wlan0"]` → `false`, slice unchanged |
| `test_patch_driver_arg_ignores_a_trailing_d_flag` | `["-D"]` with no value → `false`, no panic |

Then call `patch_driver_arg` from `main.rs` instead of the inline loop above.

### Pass criteria

- `rg 'spawn_detached' cross-compile/anyka-init/src` shows the call site still
  in `wifi.rs`, and `main.rs` contains the `killall` handover.
- Four new tests pass.
- Existing 109 lib tests and 4 integration tests still pass — the `Outcome`
  variant change touches `boot.rs`, so re-run the full suite, not a filter.
- clippy clean, fmt clean, `bash ./scripts/build_sd_contents.sh` exits 0.

### Commit

```
fix(anyka-init): hand the supplicant over to P3 with the probed driver flag

Bring-up's detached wpa_supplicant was never stopped, so the supervised
instance fought it for the ctrl socket and crash-looped. The probed -D flag was
logged and discarded, leaving the supervised instance on the hardcoded nl80211
even when wext was what associated.
```

---

## F4 — BLOCKING: shipped config is self-contradictory

**Files:**
- `SD_card_contents/anyka_hack/anyka.toml:27` and `:69-73`
- `cross-compile/anyka-init/src/config.rs:411-447`

### What is wrong

`[wifi]` ships `dhcp = false` with `address = "192.168.2.198/24"`, while
`[services.udhcpc]` ships `enabled = true` with `-f` — a persistent renewer.
The DHCP client will take a lease and replace the static address that bring-up
just assigned. The camera moves off its configured address at an unpredictable
moment after boot.

### Required fix

**1. Fix the shipped config.** Set `[services.udhcpc] enabled = false` and say
why in a comment directly above it:

```toml
[services.udhcpc]
# Disabled because [wifi] dhcp = false. A renewer would take a lease and
# replace the static address bring-up assigned. Set enabled = true only
# together with [wifi] dhcp = true.
enabled = false
```

**2. Make the contradiction unrepresentable.** Add to `Config::validate`
(`config.rs:411-447`), in the existing `if !self.wifi.dhcp { ... }` block:

```rust
    if self
        .services
        .get("udhcpc")
        .is_some_and(|s| s.enabled)
    {
        return Err(ConfigError::Invalid(
            "[services.udhcpc] is enabled but [wifi] dhcp = false; \
             the renewer would overwrite the static address"
                .into(),
        ));
    }
```

This matches the established convention in this crate: a hand-edited SD-card
config that contradicts itself is a loud failure at P1, not a silent surprise
twenty minutes into runtime.

### Tests to write first

In `config.rs`'s `mod tests`, using the existing `load_from_str` helper
(`config.rs:537`):

| Test name | Assertion |
|---|---|
| `test_config_rejects_dhcp_client_alongside_static_addressing` | `dhcp = false` + enabled `[services.udhcpc]` → `Err`, message contains `udhcpc` |
| `test_config_accepts_dhcp_client_when_dhcp_is_enabled` | `dhcp = true` + enabled `[services.udhcpc]` → `Ok` |
| `test_config_accepts_static_addressing_with_the_client_disabled` | `dhcp = false` + `enabled = false` → `Ok` |

### Pass criteria

- Three new tests pass; the full lib suite passes.
- The shipped `anyka.toml` parses: add a test or run the binary against it, and
  confirm `Config::load` on `SD_card_contents/anyka_hack/anyka.toml` succeeds.
  **This is the step that catches a botched fix** — if you disable the service
  but the new validation is inverted, this is where it shows.

### Commit

```
fix(sd): disable the DHCP renewer under static addressing, and reject the combination
```

---

## F5 — Validate CIDR and gateway at config-parse time

**Files:** `cross-compile/anyka-init/src/config.rs:427-438`, comment at
`cross-compile/anyka-init/src/wifi.rs:270-273`

### What is wrong

`parse_cidr`'s own doc comment states:

> Runs at config-parse time, not at apply time: a malformed static address is a
> W6-class failure — the camera associates, is unreachable, and no rung of the
> R7 fallback fires because there is a carrier (R12).

That is false. `Config::validate` checks only that `address` and `gateway` are
`Some`, never their shape. `address = "192.168.2.198"` (no prefix) validates
clean and fails later, inside bring-up. It does fail safe — the vendor fallback
fires — but the comment asserts a guarantee the code does not provide, and a
config error surfacing as a bring-up error costs a reboot to diagnose.

### Required fix

In the existing `if !self.wifi.dhcp { ... }` block, after the two presence
checks, validate the shapes:

- `address`: must satisfy `crate::wifi::parse_cidr(..).is_some()`.
- `gateway`: must parse as `std::net::Ipv4Addr`.

Error messages must name the offending field and its value, matching the style
of the surrounding `ConfigError::Invalid` messages.

### Tests to write first

| Test name | Assertion |
|---|---|
| `test_wifi_rejects_static_address_without_a_prefix` | `address = "192.168.2.198"` → `Err`, message contains `address` |
| `test_wifi_rejects_malformed_static_gateway` | `gateway = "192.168.2"` → `Err`, message contains `gateway` |
| `test_wifi_accepts_a_wellformed_static_configuration` | the shipped values → `Ok` |

### Pass criteria

New tests pass; full suite green; the `parse_cidr` doc comment is now true —
leave it as written rather than softening it.

### Commit

```
fix(anyka-init): validate static address and gateway shape at config-parse time
```

---

## F6 — Reset `ticks` after a reboot request

**File:** `cross-compile/anyka-init/src/monitor.rs:137-146`

### What is wrong

The `Action::Reboot` arm increments `wifi_reboots`, saves, and calls
`sys.reboot()` — but does not reset `ticks` and does not stop the loop. If
`reboot()` returns without rebooting, `ticks` is still above
`reboot_after_ticks`, so the very next tick re-enters `Reboot` and increments
the counter again. The R14 cap still bounds the damage at 3, but the budget
burns in three minutes instead of thirty, and the log gives no hint why.

Every other arm (`RunDhcp`, `RestartSupplicant`) already resets `ticks`. This
one is the odd one out.

### Required fix

Set `ticks = 0` in the `Action::Reboot` arm, after `sys.reboot()`, and log the
`reboot()` error rather than discarding it with `let _ =`. A `reboot` that
returns is itself a reportable event.

### Tests

`monitor::run` is an infinite loop and is not directly testable; the escalation
policy it drives is already covered by `netstat::decide`'s seven tests. No new
test is required — state that explicitly in the commit body rather than
inventing a test that proves nothing.

### Pass criteria

Full suite green, clippy clean.

### Commit

```
fix(anyka-init): reset the unhealthy-tick counter after a reboot request
```

---

## F7 — Seam and comment cleanup

Three small items, one commit.

**1. `cross-compile/anyka-init/src/wifi.rs:620-622`** — `gateway_reachable`
takes `_sys: &dyn Sys` and ignores it. Delete the parameter and update the call
site at `:573`.

**2. `cross-compile/anyka-init/src/wifi.rs:511-513`** — the comment on
`udhcpc_oneshot_args` says "argv[0] selects the applet". It does not: the
process is spawned as `/bin/busybox` with `udhcpc` as the first argument, so
argv[0] is `/bin/busybox` and busybox resolves the applet from argv[1]. The
behaviour is correct; the explanation is not. Rewrite it to describe what
actually happens, and keep the `/sbin/udhcpc does not exist on this device`
justification, which is accurate and useful.

The assertion in `test_udhcpc_oneshot_args_exit_after_lease`
(`wifi.rs:905-908`) carries the same wrong claim in its message. Fix the
message; keep the assertion.

**3. `cross-compile/anyka-init/src/netstat.rs:72`** — `gateway_reachable` calls
`std::thread::sleep(200ms)` directly, blocking the monitor thread on every tick.
This matches the source plan, so it is not a regression. Leave the behaviour
alone and add a `ponytail:` comment naming the ceiling:

```rust
    // ponytail: blocks the caller 200 ms per probe. Fine at a 60 s tick; move
    // to Sys::sleep if the monitor ever needs a faster loop.
```

### Pass criteria

Full suite green, clippy clean, fmt clean. No behaviour change.

### Commit

```
chore(anyka-init): drop an unused seam parameter and correct two misleading comments
```

---

## Final Verification — run all of this before you report done

```bash
source ./setenv.sh
cd cross-compile/anyka-init

$CARGO test --target x86_64-unknown-linux-gnu --lib
$CARGO test --target x86_64-unknown-linux-gnu --test supervision -- --test-threads=1
$CARGO clippy --target x86_64-unknown-linux-gnu --all-targets -- -D warnings
$CARGO fmt --check

cd "$(git rev-parse --show-toplevel)"
bash ./scripts/build_sd_contents.sh
```

Expected:

| Check | Expected |
|---|---|
| Lib tests | **≥ 125 passed, 0 failed** (109 on arrival + ~16 new) |
| Integration tests | 4 passed, 0 failed |
| clippy | exit 0, no warnings |
| fmt | exit 0 |
| Cross-compile | exit 0, `anyka-init installed to .../anyka_hack/anyka-init.bin` |

Then confirm, individually:

- `rg 'fn read_address' cross-compile/anyka-init/src` → no matches (F1)
- `rg 'killall' cross-compile/anyka-init/src/main.rs` → the handover exists (F2)
- `rg 'driver' cross-compile/anyka-init/src/boot.rs` → the flag is threaded (F3)
- `rg -A2 '\[services.udhcpc\]' SD_card_contents/anyka_hack/anyka.toml` →
  `enabled = false` (F4)
- **6 commits** on the branch: F1, F2+F3 (one commit), F4, F5, F6, F7.

**Do not claim any of the above passes without pasting the command output.**
"Should pass" is not a result. If something fails, report the failure and the
output; a truthful red is worth more than a confident green.

---

## Explicitly out of scope

- Hardware validation. `A10` and `B7` in
  `docs/plans/2026-08-01-wifi-bring-up.md` need the physical camera and a
  known-good recovery SD card. Do not attempt them.
- The `W7` and `R17` open questions in
  `docs/plans/2026-08-01-boot-runtime-rust-design.md` — both need hardware
  answers.
- Anything in `orig/`. It is a read-only vendor capture and your specification.
- The `vendor-daemon.bin` and `onvif-rust.bin` diffs already in the working
  tree. They are unrelated build artifacts; leave them alone.
