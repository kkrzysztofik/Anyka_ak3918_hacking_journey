# anyka-init ponytail cleanup — design

Date: 2026-10-04
Branch: `refactor/anyka-init-ponytail-cleanup`
Scope: `cross-compile/anyka-init/` only.

## Goal

Delete duplication and dead flexibility found by a ponytail review of the
whole crate (~45 findings, estimated -600 of 12.5k lines) without changing
what a camera does, except for four deliberate behaviour changes listed below.

The crate is already well structured (pure decision functions, one `Sys`
syscall boundary). The excess is copies, not design: every config default is
written twice, the atomic-write dance three times, the control-socket reply
mapping twice, and the same test `Config` literal in three modules.

## Decisions

| Question | Decision |
|---|---|
| Behaviour-changing findings | All four in, each in its own commit |
| Delivery | One PR, one commit per theme, every commit green |
| Verification | Host tests + clippy per commit; ARM cross-build; A/B bundle deploy to .198 |

### The four behaviour changes

1. **Overlay validation.** `NetworkOverlay::validate` is deleted; the
   post-merge `cfg.validate()` already quarantines a bad result. New
   semantics: an overlay with `dhcp = false` and no `address` is accepted when
   the baseline supplies the address. A test pins this.
2. **Nonce.** `random_nonce` becomes `RandomState::new().hash_one(())` — the
   same OS randomness source. It panics instead of degrading if
   `/dev/urandom` is absent; `fallback_nonce` and its test go.
3. **loadavg.** `parse_loadavg` and the `load1` field leave the `sys` log
   line. loadavg is meaningless on this SoC (D-state wifi threads).
4. **Untar.** `busybox tar -xf … -C …` is invoked directly instead of via
   `sh -c`. `shell_quote` stays for the `sha256sum` `cd`.

## Commit sequence

| # | Theme | Content | Δ est. |
|---|---|---|---|
| 0 | Pinning tests | Each section `Default` == minimal-TOML parse; `backoff_delay` sweep; `periodic_reboot_delay` at jitter 0 | +40 |
| 1 | Test helpers | One TOML-backed `#[cfg(test)] test_config()`; `rewrite_env`, `happy_layout`, log-poll helpers; delete the empty TZ spawn test and the determinism test | -115 |
| 2 | Config defaults | Container `#[serde(default)]`, drop 33 `d_*` fns; `parse_file` → `read_config_text`; `edit_file`; drop `set_bool_in_text` | -155 |
| 3 | Shared I/O helpers | `atomic_write` (storm/update/config); `spawn_named` (8 thread sites) | -40 |
| 4 | Supervisor loop | `make_channel`, single-variant `ControlMsg`, `signal_service`, one reply path for toggle + set-ntp, `ctx()`, `Service::pending`, pending status row, jitter, deadline min, misplaced doc comment | -95 |
| 5 | Small module cuts | supervise, control, storm, monitor (`TickState`, `VideoPolicy`, `sample_link`), netstat, update (`revert`, `ports` param), sys (`ManuallyDrop`), wifi (`bring_up*`, constants, wrapper), timesync (`first_sync`, `sync_once`), netoverlay (`Serialize`, `has_content`), `config-check` example | -165 |
| 6 | Overlay validate | Behaviour change 1 + test | -25 |
| 7 | RandomState nonce | Behaviour change 2 | -25 |
| 8 | loadavg | Behaviour change 3 | -10 |
| 9 | Untar | Behaviour change 4 | -4 |

Commit 0 exists because commit 2 is the dangerous one: a default value that
silently changes while moving from a `d_*` fn into a `Default` impl would only
show up on a camera. Pinning tests make that a red host test instead.

Ordering puts test helpers before code so later commits carry smaller test
diffs, and the behaviour changes last so a misbehaving deploy can be bisected
to one revertable commit.

## Constraints

- **No new `anyka.toml` keys.** `deny_unknown_fields` makes a new key a hard
  parse error for the older anyka-init in the other A/B slot. Container-level
  `#[serde(default)]` adds no keys, so it is rollback-safe.
- **Defaults are byte-identical.** Commit 0 enforces it.
- **No public-API breakage outside the crate.** `control::SOCKET_PATH`, the
  TSV wire format and the `ok/unknown/error/pending` replies are consumed by
  onvif-rust's `diagnostics/services.rs` and stay unchanged.
- **Keep `RealSys: Default`.** Removing it trips clippy `new_without_default`.

## Verification

Per commit, from `cross-compile/` with the vendored toolchain first on PATH:

- `cargo test --target x86_64-unknown-linux-gnu -p anyka-init`
- `cargo clippy --target x86_64-unknown-linux-gnu -p anyka-init --all-targets -- -D warnings`
- `cargo fmt -p anyka-init --check`

Before the PR:

- ARM cross-build from `cross-compile/anyka-init/` (PR CI never builds ARM).
- A/B bundle to .198; confirm boot with defaults, supervisor status over the
  control socket, an NTP sync and `ntp.status`, trial confirm (exercises the
  untar change), and that the `sys` log line no longer carries `load1`.

Baseline at branch point: 352 tests passing, clippy clean.

## Out of scope

Correctness findings, and anything in onvif-rust (including its own
`NetworkOverlay` copy).
