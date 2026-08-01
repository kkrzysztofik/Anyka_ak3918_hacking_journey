# Wifi Bring-Up and Link Monitoring Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the vendor's 1,316-line shell wifi chain with a Rust
implementation in `anyka-init`, then add link monitoring that notices when the
connection stops working.

**Architecture:** Two independent phases. **Phase A** adds `src/wifi.rs` and
replaces the `wifi_manage.sh start` call at `boot.rs:104` with `wifi::bring_up`,
keeping the vendor script as a mandatory fallback. **Phase B** extends the
existing 60-second monitor thread with link health checks and a bounded
escalation ladder. Phase B touches no boot-path code and can ship separately.

Risk is concentrated in Phase A because wifi is the camera's only recovery
channel (finding W6). Every decision that can be a pure function is one, so the
dispatch table, credential rules, generated config and escalation policy are all
provable on `x86_64` without hardware.

**Tech Stack:** Rust 2024, `std` only (no tokio), `serde`/`toml` for config,
`mockall` for the `Sys` seam, vendored toolchain at
`toolchain/arm-anykav200-crosstool-ng/bin/cargo`.

**Design doc:** `docs/plans/2026-08-01-boot-runtime-rust-design.md`, section
*Addendum: Wifi Bring-Up in Rust*. Read it before starting. Findings W1–W7 and
risks R7–R17 are referenced by number throughout this plan.

---

## Before You Start

### Toolchain

Every `cargo` invocation must use the vendored toolchain. From the repo root:

```bash
source ./setenv.sh
```

This exports `$CARGO` and prepends the toolchain `bin/` to `PATH`. The `PATH`
prefix is not optional — `cargo clippy` fails with `E0514` without it.

### Commands you will use constantly

```bash
cd cross-compile/anyka-init

# Unit tests (pure functions) — this is what you run after almost every step
$CARGO test --target x86_64-unknown-linux-gnu --lib

# One test by name
$CARGO test --target x86_64-unknown-linux-gnu --lib test_chip_from_hw_char_h

# Integration tests — MUST be single-threaded; they call waitpid(-1)
$CARGO test --target x86_64-unknown-linux-gnu --test supervision -- --test-threads=1

# Lint and format, both required before any commit
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO fmt --check
```

### Conventions this codebase already follows

Match them; do not invent new ones.

- Pure functions live next to their impure caller in the same module, with a
  `#[cfg(test)] mod tests` block at the bottom of the file. See
  `src/boot.rs:122-174` and `src/monitor.rs:49-82`.
- Test names are `test_<subject>_<behaviour>`, never `test1` or `test_init`.
- No `unwrap()` or `expect()` outside tests. Use `Result` and `?`.
- All syscalls go through the `Sys` trait (`src/sys.rs:47`) so `MockSys` can
  stand in. Plain file reads and writes do **not** — `apply_wifi` at
  `src/boot.rs:43` uses `std::fs` directly and that is the established pattern.
- `#[serde(deny_unknown_fields)]` on every config struct. A typo in a
  hand-edited SD-card config must be a loud failure.
- Comments explain *why*, and cite vendor source as `file:line` when the
  behaviour is transcribed from it.

### A note on the vendor scripts

They are your specification. When this plan says "transcribe", it means read the
cited lines and reproduce the behaviour exactly — do not improve it, do not
guess at values. The files are in `orig/`:

| File | What it defines |
|---|---|
| `orig/data/wifi_driver.sh` | hw.conf offsets (33-47), GPIO sequence (370-387), the ten `insmod` variants (240-370) |
| `orig/data/wifi_station.sh` | `wpa_supplicant` invocation (55-68), DHCP branch (85-110) |
| `orig/usr/sbin/wifi_run.sh` | overall sequencing (185-242) |
| `orig/usr/sbin/station_connect.sh` | `wpa_cli` association path (57-95) |

---

# Phase A — Bring-Up

Nine tasks. At the end of Phase A the camera associates and gets an address
without executing any vendor wifi script, and falls back to the vendor chain if
it cannot.

---

### Task A1: Extend `WifiCfg`

**Files:**
- Modify: `cross-compile/anyka-init/src/config.rs:82-87`
- Test: same file, existing `#[cfg(test)] mod tests` block

The current struct has three fields. It needs the full schema from the design's
*Config additions* section.

**Step 1: Write the failing tests**

Add to the tests module in `src/config.rs`:

```rust
#[test]
fn test_wifi_defaults_are_dhcp_and_auto_chip() {
    let cfg: Config = toml::from_str(
        r#"
[wifi]
ssid = "net"
password = "secret12"
"#,
    )
    .expect("parse");
    assert!(cfg.wifi.dhcp);
    assert_eq!(cfg.wifi.chip, "auto");
    assert_eq!(cfg.wifi.interface, "wlan0");
    assert!(cfg.wifi.fallback_to_vendor);
}

#[test]
fn test_wifi_static_requires_address_and_gateway() {
    let err = load_from_str(
        r#"
[wifi]
ssid = "net"
password = "secret12"
dhcp = false
"#,
    )
    .expect_err("static config without an address must be rejected");
    assert!(
        format!("{err}").contains("address"),
        "error should name the missing field, got: {err}"
    );
}

#[test]
fn test_wifi_rejects_unknown_key() {
    let err = load_from_str(
        r#"
[wifi]
ssid = "net"
password = "secret12"
sid = "typo"
"#,
    )
    .expect_err("deny_unknown_fields must reject a typo");
    assert!(format!("{err}").contains("sid"));
}
```

`load_from_str` is a helper that parses and then runs validation. If the tests
module does not already have one, add it:

```rust
#[cfg(test)]
fn load_from_str(src: &str) -> Result<Config, ConfigError> {
    let cfg: Config = toml::from_str(src)?;
    cfg.validate()?;
    Ok(cfg)
}
```

**Step 2: Run the tests, confirm they fail**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib test_wifi_
```

Expected: compile error — `no field 'dhcp' on type 'WifiCfg'`.

**Step 3: Extend the struct**

Replace `src/config.rs:82-87`:

```rust
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WifiCfg {
    pub ssid: String,
    pub password: String,
    #[serde(default = "d_wifi_cfg_file")]
    pub config_file: String,

    /// `auto` parses `/etc/jffs2/hw.conf`; any other value pins the chip and
    /// skips detection entirely. Pinned is the shipped default because
    /// `hw.conf` offset stability across camera revisions is unverifiable from
    /// a single board (design Q4), and because `service.sh:124` writes a
    /// 32-character default record that cannot be indexed at offset 51 (W2).
    #[serde(default = "d_wifi_chip")]
    pub chip: String,
    /// `high_low` matches vendor `WIFI_ENABLE_VALUE == "2"`
    /// (`wifi_driver.sh:374-382`); `low_high` is every other value.
    #[serde(default = "d_wifi_polarity")]
    pub gpio_polarity: String,
    #[serde(default = "d_wifi_interface")]
    pub interface: String,
    #[serde(default = "d_wifi_security")]
    pub security: String,

    #[serde(default = "d_true")]
    pub dhcp: bool,
    /// CIDR, e.g. `192.168.2.198/24`. Required when `dhcp = false`.
    #[serde(default)]
    pub address: Option<String>,
    /// Required when `dhcp = false`.
    #[serde(default)]
    pub gateway: Option<String>,
    /// Written to `/etc/resolv.conf` after the address is assigned (W7).
    #[serde(default)]
    pub dns: Vec<String>,

    #[serde(default = "d_wifi_timeout")]
    pub connect_timeout_sec: u64,
    /// R7. Not behind a flag by default: a wrong dispatch entry would
    /// otherwise cost the camera's only remote access.
    #[serde(default = "d_true")]
    pub fallback_to_vendor: bool,
}

fn d_wifi_chip() -> String {
    "auto".into()
}
fn d_wifi_polarity() -> String {
    "low_high".into()
}
fn d_wifi_interface() -> String {
    "wlan0".into()
}
fn d_wifi_security() -> String {
    "wpa".into()
}
fn d_wifi_timeout() -> u64 {
    45
}
```

**Step 4: Add validation**

Find the existing `Config::validate` (or add one if absent, called from
`Config::load`) and add:

```rust
if !self.wifi.dhcp {
    if self.wifi.address.is_none() {
        return Err(ConfigError::Invalid(
            "[wifi] address is required when dhcp = false".into(),
        ));
    }
    if self.wifi.gateway.is_none() {
        return Err(ConfigError::Invalid(
            "[wifi] gateway is required when dhcp = false".into(),
        ));
    }
}
```

**Step 5: Run the tests, confirm they pass**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib test_wifi_
```

Expected: 3 passed.

**Step 6: Commit**

```bash
git add cross-compile/anyka-init/src/config.rs
git commit -m "feat(anyka-init): extend [wifi] config for chip pinning and static addressing"
```

---

### Task A2: The chip dispatch table

**Files:**
- Create: `cross-compile/anyka-init/src/wifi.rs`
- Modify: `cross-compile/anyka-init/src/lib.rs` (add `pub mod wifi;`)

This is the whole point of the rewrite (finding W1). The vendor builds a
function name from a string — `wifi_config_${WIFI_NAME} 1` at
`wifi_driver.sh:386` — so two of ten entries are already dead and resolve to
"command not found" with no diagnostic. An exhaustive `match` makes that
unrepresentable.

**Step 1: Write the failing tests**

Create `src/wifi.rs` containing only the test module for now:

```rust
//! Wifi bring-up, replacing the vendor chain: wifi_manage.sh -> wifi_run.sh ->
//! wifi_driver.sh / wifi_station.sh -> station_connect.sh (1,316 lines of sh).
//!
//! See docs/plans/2026-08-01-boot-runtime-rust-design.md, addendum.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chip_from_hw_char_h_is_ssv6355() {
        let c = Chip::from_hw_char('h').expect("h is a known chip");
        assert_eq!(c.name, "ssv6355_ble");
        assert_eq!(c.module, "/tmp/ko/ssv6355.ko");
        assert_eq!(c.args, "stacfgpath=/tmp/ko/ssv6355-wifi.cfg");
        // Three different SSV modules all unload under this one name; the
        // rmmod name is not derivable from the .ko filename.
        assert_eq!(c.rmmod, "ssv6x5x");
    }

    #[test]
    fn test_chip_from_hw_char_rejects_dead_vendor_paths() {
        // Vendor types 3 and 4 build function names that do not exist
        // (wifi_config_rtl8189, and wifi_config_atbm603x_HT20 vs the defined
        // wifi_config_atbm603_HT20). They must be an explicit None here, not a
        // silent no-op (W1).
        assert!(Chip::from_hw_char('3').is_none());
        assert!(Chip::from_hw_char('4').is_none());
    }

    #[test]
    fn test_chip_from_hw_char_rejects_unknown() {
        assert!(Chip::from_hw_char('z').is_none());
        assert!(Chip::from_hw_char('\0').is_none());
    }

    #[test]
    fn test_chip_from_name_round_trips_every_entry() {
        for c in Chip::ALL {
            let looked_up = Chip::from_name(c.name).expect("every name resolves");
            assert_eq!(looked_up.name, c.name);
        }
    }

    #[test]
    fn test_chip_from_name_rejects_unknown() {
        assert!(Chip::from_name("nonexistent").is_none());
        assert!(Chip::from_name("").is_none());
    }
}
```

**Step 2: Run, confirm it fails**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib test_chip_
```

Expected: `cannot find type 'Chip' in this scope`.

**Step 3: Implement the table**

Add above the test module in `src/wifi.rs`. Transcribe every row from
`orig/data/wifi_driver.sh:240-370`. `WIFI_DRIVER_PATH` and `WIFI_CFG_PATH` are
both `/tmp/ko` (`wifi_driver.sh:104-105`) — that is why the loaded module is not
in `/usr/modules`.

```rust
use std::time::Duration;

/// One row of the vendor's chip dispatch, transcribed from
/// `orig/data/wifi_driver.sh:240-370`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chip {
    /// Vendor `WIFI_NAME`. Also the accepted value of `[wifi].chip`.
    pub name: &'static str,
    pub module: &'static str,
    /// Module parameters, empty when the vendor passes none.
    pub args: &'static str,
    /// Not derivable from `module`: three SSV variants unload as `ssv6x5x`,
    /// and `txw801.ko` unloads as `hgics`.
    pub rmmod: &'static str,
    /// Vendor `sleep` following the insmod, where it has one.
    pub settle: Duration,
}

impl Chip {
    pub const ALL: &'static [Chip] = &[
        Chip {
            name: "ssv6x5x",
            module: "/tmp/ko/ssv6x5x.ko",
            args: "stacfgpath=/tmp/ko/ak3916-wifi.cfg",
            rmmod: "ssv6x5x",
            settle: Duration::ZERO,
        },
        Chip {
            name: "rtl8188ftv_new",
            module: "/tmp/ko/rtl8188fu.ko",
            args: "",
            rmmod: "rtl8188fu",
            settle: Duration::ZERO,
        },
        Chip {
            name: "rda5995",
            module: "/tmp/ko/rdawfmac.ko",
            args: "",
            rmmod: "rdawfmac",
            settle: Duration::ZERO,
        },
        Chip {
            name: "txw801",
            module: "/tmp/ko/txw801.ko",
            args: "fw_file=txw801x_USB.bin",
            rmmod: "hgics",
            settle: Duration::from_secs(2),
        },
        Chip {
            name: "rtl8731_8733",
            module: "/tmp/ko/8733bu.ko",
            args: "",
            rmmod: "8733bu",
            settle: Duration::ZERO,
        },
        Chip {
            name: "ssv6115_wifi6",
            module: "/tmp/ko/ssv6x5x_wifi6.ko",
            args: "stacfgpath=/tmp/ko/ak3916-wifi6.cfg",
            rmmod: "ssv6x5x",
            settle: Duration::ZERO,
        },
        Chip {
            name: "zt9101",
            module: "/tmp/ko/ZT9101UV20.ko",
            args: "cfg=/tmp/ko/wifi.cfg",
            rmmod: "ZT9101UV20",
            settle: Duration::ZERO,
        },
        Chip {
            name: "ssv6355_ble",
            module: "/tmp/ko/ssv6355.ko",
            args: "stacfgpath=/tmp/ko/ssv6355-wifi.cfg",
            rmmod: "ssv6x5x",
            settle: Duration::ZERO,
        },
    ];

    /// Maps the `hw.conf` chip character to a row.
    ///
    /// Vendor types `3` (rtl8189) and `4` (atbm603x_HT20) are deliberately
    /// absent: both dispatch to shell function names that do not exist, so the
    /// vendor never loads a driver for them and `wifi_run.sh` then hangs on the
    /// empty-SSID branch (W1). Returning `None` turns that into a loud error
    /// and the R7 fallback.
    pub fn from_hw_char(c: char) -> Option<&'static Chip> {
        let name = match c {
            '1' => "ssv6x5x",
            '2' => "rtl8188ftv_new",
            '7' => "rda5995",
            'd' => "txw801",
            'e' => "rtl8731_8733",
            'f' => "ssv6115_wifi6",
            'g' => "zt9101",
            'h' => "ssv6355_ble",
            _ => return None,
        };
        Self::from_name(name)
    }

    pub fn from_name(name: &str) -> Option<&'static Chip> {
        Self::ALL.iter().find(|c| c.name == name)
    }

    /// insmod argv, ready for `Sys::run_to_completion`.
    pub fn insmod_args(&self) -> Vec<String> {
        let mut v = vec![self.module.to_string()];
        if !self.args.is_empty() {
            v.push(self.args.to_string());
        }
        v
    }
}
```

Then add `pub mod wifi;` to `src/lib.rs`.

**Step 4: Run, confirm pass**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib test_chip_
```

Expected: 5 passed.

**Step 5: Commit**

```bash
git add cross-compile/anyka-init/src/wifi.rs cross-compile/anyka-init/src/lib.rs
git commit -m "feat(anyka-init): exhaustive wifi chip dispatch table

Replaces the vendor's string-built function dispatch (wifi_driver.sh:386),
where two of ten entries resolve to command-not-found with no diagnostic."
```

---

### Task A3: Parse `hw.conf`

**Files:**
- Modify: `cross-compile/anyka-init/src/wifi.rs`

**Step 1: Write the failing tests**

```rust
// The real record from this camera: orig/etc/jffs2/hw.conf, byte-identical to
// the /mnt/Factory copy. 64 characters after the HW= prefix.
const HW_REAL: &str =
    "HW=111513155011100180020000000000000000000000020000003h200000000000\n";

// service.sh:124 writes this when hw.conf is absent: 32 characters, so
// offset 51 does not exist (W2).
const HW_DEFAULT: &str = "HW=12151005501110018000000000000000\n";

#[test]
fn test_parse_hw_conf_extracts_chip_and_polarity() {
    let hw = parse_hw_conf(HW_REAL).expect("real record parses");
    assert_eq!(hw.chip_char, 'h');
    assert_eq!(hw.polarity_char, '2');
}

#[test]
fn test_parse_hw_conf_rejects_short_default_record() {
    // Bash `${HW_READ:51:1}` yields "" here and dispatches wifi_config_ with no
    // diagnostic. Option makes that unignorable.
    assert!(parse_hw_conf(HW_DEFAULT).is_none());
}

#[test]
fn test_parse_hw_conf_rejects_missing_prefix() {
    assert!(parse_hw_conf("111513155011100180020000000000000000000000020000003h2").is_none());
    assert!(parse_hw_conf("").is_none());
}

#[test]
fn test_parse_hw_conf_tolerates_missing_trailing_newline() {
    let hw = parse_hw_conf(HW_REAL.trim_end()).expect("no trailing newline");
    assert_eq!(hw.chip_char, 'h');
}

#[test]
fn test_hw_polarity_maps_two_to_high_low() {
    assert_eq!(Polarity::from_char('2'), Polarity::HighLow);
    assert_eq!(Polarity::from_char('1'), Polarity::LowHigh);
    assert_eq!(Polarity::from_char('x'), Polarity::LowHigh);
}
```

**Step 2: Run, confirm failure**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib test_parse_hw_conf
```

Expected: `cannot find function 'parse_hw_conf'`.

**Step 3: Implement**

```rust
/// Vendor offsets, from `wifi_driver.sh:41-47`. Both are zero-based indices
/// into the record *after* the three-byte `HW=` prefix is dropped, matching
/// bash `${HW_READ:51:1}`.
const HW_CHIP_OFFSET: usize = 51;
const HW_POLARITY_OFFSET: usize = 52;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HwConf {
    pub chip_char: char,
    pub polarity_char: char,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Polarity {
    /// Vendor `WIFI_ENABLE_VALUE == "2"`: 1 then 0 (`wifi_driver.sh:374-377`).
    HighLow,
    /// Everything else: 0 then 1 (`wifi_driver.sh:378-381`).
    LowHigh,
}

impl Polarity {
    pub fn from_char(c: char) -> Self {
        if c == '2' { Self::HighLow } else { Self::LowHigh }
    }

    pub fn from_name(s: &str) -> Option<Self> {
        match s {
            "high_low" => Some(Self::HighLow),
            "low_high" => Some(Self::LowHigh),
            _ => None,
        }
    }

    /// The two values to write to `/sys/user-gpio/wifi_en`, in order.
    pub fn sequence(&self) -> [&'static str; 2] {
        match self {
            Self::HighLow => ["1", "0"],
            Self::LowHigh => ["0", "1"],
        }
    }
}

/// Parse `/etc/jffs2/hw.conf`. `None` for anything the vendor would have
/// silently turned into an empty `WIFI_NAME`.
pub fn parse_hw_conf(src: &str) -> Option<HwConf> {
    let record = src.trim_end_matches(['\n', '\r']).strip_prefix("HW=")?;
    let chars: Vec<char> = record.chars().collect();
    Some(HwConf {
        chip_char: *chars.get(HW_CHIP_OFFSET)?,
        polarity_char: *chars.get(HW_POLARITY_OFFSET)?,
    })
}
```

**Step 4: Run, confirm pass**

Expected: 5 passed.

**Step 5: Commit**

```bash
git add cross-compile/anyka-init/src/wifi.rs
git commit -m "feat(anyka-init): parse hw.conf chip and GPIO polarity offsets"
```

---

### Task A4: Credential validation and `wpa_supplicant.conf`

**Files:**
- Modify: `cross-compile/anyka-init/src/wifi.rs`

Finding W3: `station_connect.sh:89-91` interpolates credentials into `sh -c`, so
a `"`, `$`, backtick or `\` breaks the quoting. This is the documented "some
special characters don't work in wifi ssid names and passwords".

Read R11 carefully before writing this. The fix is **not** to reject shell
metacharacters — those are legal in a PSK and only broke because the vendor went
through a shell. Reject only what cannot survive the `wpa_supplicant.conf`
grammar.

**Step 1: Write the failing tests**

```rust
#[test]
fn test_validate_credentials_accepts_shell_metacharacters() {
    // R11: these broke the vendor only because it went through `sh -c`. They
    // are legal in a PSK and rejecting them would lock a user out of their
    // own network.
    for psk in [r#"a$b`c\d;e&f"#, "pass word", "'quoted'", "12345678"] {
        assert!(
            validate_credentials("net", psk, Security::Wpa).is_ok(),
            "must accept {psk:?}"
        );
    }
}

#[test]
fn test_validate_credentials_rejects_ungrammatical_characters() {
    for psk in ["has\"quote", "has\nnewline", "has\0nul"] {
        assert!(
            validate_credentials("net", psk, Security::Wpa).is_err(),
            "must reject {psk:?}"
        );
    }
    assert!(validate_credentials("has\"quote", "goodpass", Security::Wpa).is_err());
}

#[test]
fn test_validate_credentials_enforces_wpa_psk_length() {
    assert!(validate_credentials("net", "short7c", Security::Wpa).is_err());
    assert!(validate_credentials("net", &"x".repeat(64), Security::Wpa).is_err());
    assert!(validate_credentials("net", &"x".repeat(63), Security::Wpa).is_ok());
}

#[test]
fn test_validate_credentials_enforces_ssid_length() {
    assert!(validate_credentials("", "goodpass", Security::Wpa).is_err());
    assert!(validate_credentials(&"s".repeat(33), "goodpass", Security::Wpa).is_err());
    assert!(validate_credentials(&"s".repeat(32), "goodpass", Security::Wpa).is_ok());
}

#[test]
fn test_validate_credentials_open_ignores_psk_length() {
    assert!(validate_credentials("net", "", Security::Open).is_ok());
}

#[test]
fn test_wpa_supplicant_conf_quotes_ssid_and_psk() {
    let out = wpa_supplicant_conf("my net", "s3cret!!", Security::Wpa);
    assert!(out.contains("ctrl_interface="), "wpa_cli needs a control socket");
    assert!(out.contains(r#"ssid="my net""#));
    assert!(out.contains(r#"psk="s3cret!!""#));
    assert!(out.contains("key_mgmt=WPA-PSK"));
}

#[test]
fn test_wpa_supplicant_conf_open_network_has_no_psk() {
    let out = wpa_supplicant_conf("guest", "", Security::Open);
    assert!(out.contains("key_mgmt=NONE"));
    assert!(!out.contains("psk="));
}

#[test]
fn test_wpa_supplicant_conf_is_deterministic() {
    let a = wpa_supplicant_conf("net", "password", Security::Wpa);
    let b = wpa_supplicant_conf("net", "password", Security::Wpa);
    assert_eq!(a, b);
}
```

**Step 2: Run, confirm failure**

**Step 3: Implement**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Security {
    Wpa,
    Wep,
    Open,
}

impl Security {
    pub fn from_name(s: &str) -> Option<Self> {
        match s {
            "wpa" => Some(Self::Wpa),
            "wep" => Some(Self::Wep),
            "open" => Some(Self::Open),
            _ => None,
        }
    }
}

/// Characters that cannot survive the `wpa_supplicant.conf` grammar, where
/// values are double-quoted strings on a single line.
///
/// Deliberately short. Shell metacharacters (`$`, backtick, `\`, `;`, `&`) are
/// accepted: they broke `station_connect.sh:89-91` only because it built a
/// command line for `sh -c`, and they are legal in a PSK (R11).
const UNGRAMMATICAL: [char; 3] = ['"', '\n', '\0'];

pub fn validate_credentials(ssid: &str, psk: &str, sec: Security) -> Result<(), String> {
    if ssid.is_empty() {
        // W4: a blank SSID sends wifi_run.sh:188 into a 1 Hz wait for a file
        // that only anyka_ipc writes, and anyka_ipc never runs under
        // FACTORY_TEST=1. It hangs forever, silently.
        return Err("[wifi] ssid is empty".into());
    }
    if ssid.len() > 32 {
        return Err(format!("[wifi] ssid is {} bytes, max 32", ssid.len()));
    }
    if let Some(c) = ssid.chars().find(|c| UNGRAMMATICAL.contains(c)) {
        return Err(format!("[wifi] ssid contains unsupported character {c:?}"));
    }
    if let Some(c) = psk.chars().find(|c| UNGRAMMATICAL.contains(c)) {
        return Err(format!("[wifi] password contains unsupported character {c:?}"));
    }
    if sec == Security::Wpa && !(8..=63).contains(&psk.len()) {
        return Err(format!(
            "[wifi] WPA password is {} bytes, must be 8..=63",
            psk.len()
        ));
    }
    Ok(())
}

/// Generate a single-network `wpa_supplicant.conf`.
///
/// Replaces both vendor mechanisms at once: the line-numbered `sed` into lines
/// 3 and 4 (`wifi_station.sh:51-54`) and the `wpa_cli set_network` path
/// (`station_connect.sh:57-95`).
pub fn wpa_supplicant_conf(ssid: &str, psk: &str, sec: Security) -> String {
    let mut s = String::with_capacity(256);
    s.push_str("ctrl_interface=/var/run/wpa_supplicant\n");
    s.push_str("update_config=1\n\n");
    s.push_str("network={\n");
    s.push_str(&format!("\tssid=\"{ssid}\"\n"));
    match sec {
        Security::Wpa => {
            s.push_str("\tkey_mgmt=WPA-PSK\n");
            s.push_str(&format!("\tpsk=\"{psk}\"\n"));
        }
        Security::Wep => {
            s.push_str("\tkey_mgmt=NONE\n");
            s.push_str("\twep_tx_keyidx=0\n");
            s.push_str(&format!("\twep_key0=\"{psk}\"\n"));
        }
        Security::Open => {
            s.push_str("\tkey_mgmt=NONE\n");
        }
    }
    s.push_str("}\n");
    s
}
```

> The generated file is safe to interpolate because `validate_credentials` has
> already removed every character that could terminate the quoted string. Call
> them in that order; never generate a conf for unvalidated credentials.

**Step 4: Run, confirm pass** — 8 passed.

**Step 5: Commit**

```bash
git add cross-compile/anyka-init/src/wifi.rs
git commit -m "feat(anyka-init): credential validation and wpa_supplicant.conf generation

Fixes W3: credentials no longer transit a shell, so shell metacharacters are
accepted rather than silently breaking association."
```

---

### Task A5: Address parsing and `resolv.conf`

**Files:**
- Modify: `cross-compile/anyka-init/src/wifi.rs`

**Step 1: Write the failing tests**

```rust
#[test]
fn test_parse_cidr_splits_address_and_netmask() {
    let a = parse_cidr("192.168.2.198/24").expect("valid cidr");
    assert_eq!(a.address, "192.168.2.198");
    assert_eq!(a.netmask, "255.255.255.0");
}

#[test]
fn test_parse_cidr_handles_uncommon_prefixes() {
    assert_eq!(parse_cidr("10.0.0.1/8").expect("/8").netmask, "255.0.0.0");
    assert_eq!(parse_cidr("10.0.0.1/30").expect("/30").netmask, "255.255.255.252");
    assert_eq!(parse_cidr("10.0.0.1/32").expect("/32").netmask, "255.255.255.255");
}

#[test]
fn test_parse_cidr_rejects_malformed_input() {
    // R12: every one of these, accepted, produces an unreachable camera.
    for bad in [
        "192.168.2.198",     // no prefix
        "192.168.2.198/33",  // prefix out of range
        "192.168.2/24",      // too few octets
        "192.168.2.999/24",  // octet out of range
        "not-an-address/24",
        "",
    ] {
        assert!(parse_cidr(bad).is_none(), "must reject {bad:?}");
    }
}

#[test]
fn test_resolv_conf_renders_one_nameserver_per_line() {
    let out = resolv_conf(&["192.168.2.1".into(), "8.8.8.8".into()]);
    assert_eq!(out, "nameserver 192.168.2.1\nnameserver 8.8.8.8\n");
}

#[test]
fn test_resolv_conf_empty_list_is_empty_string() {
    assert_eq!(resolv_conf(&[]), "");
}
```

**Step 2: Run, confirm failure**

**Step 3: Implement**

```rust
use std::net::Ipv4Addr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cidr {
    pub address: String,
    pub netmask: String,
}

/// Parse `a.b.c.d/prefix` into the two strings `ifconfig` wants.
///
/// Runs at config-parse time, not at apply time: a malformed static address is
/// a W6-class failure — the camera associates, is unreachable, and no rung of
/// the R7 fallback fires because there is a carrier (R12).
pub fn parse_cidr(src: &str) -> Option<Cidr> {
    let (addr, prefix) = src.split_once('/')?;
    let addr: Ipv4Addr = addr.parse().ok()?;
    let prefix: u32 = prefix.parse().ok()?;
    if prefix > 32 {
        return None;
    }
    let mask = if prefix == 0 {
        0u32
    } else {
        u32::MAX << (32 - prefix)
    };
    Some(Cidr {
        address: addr.to_string(),
        netmask: Ipv4Addr::from(mask).to_string(),
    })
}

/// W7: the resolver reads `/etc/resolv.conf`, but busybox udhcpc writes
/// `/etc/jffs2/resolv.conf` (`/usr/share/udhcpc/default.script:5`) and nothing
/// in the rootfs links the two. Without this, SNTP by hostname never resolves
/// and D7 fires — a wrong clock rejects every authenticated ONVIF request.
pub fn resolv_conf(servers: &[String]) -> String {
    servers
        .iter()
        .map(|s| format!("nameserver {s}\n"))
        .collect()
}
```

**Step 4: Run, confirm pass** — 5 passed.

**Step 5: Commit**

```bash
git add cross-compile/anyka-init/src/wifi.rs
git commit -m "feat(anyka-init): CIDR parsing and resolv.conf generation"
```

---

### Task A6: Extend the `Sys` seam

**Files:**
- Modify: `cross-compile/anyka-init/src/sys.rs:47-65` (trait) and `86-231` (impl)

Bring-up needs two things the trait does not expose: unloading a module, and
sleeping under mock control. Everything else is `run_to_completion`, `insmod`,
or plain `std::fs` (following `apply_wifi`'s precedent at `src/boot.rs:43`).

**Step 1: Add to the trait**

```rust
    fn rmmod(&self, name: &str) -> Result<(), SysError>;
    /// Mockable sleep. Bring-up has several vendor-transcribed settle delays;
    /// a real `thread::sleep` would make the bring-up tests take 40 seconds.
    fn sleep(&self, d: Duration);
```

**Step 2: Implement on `RealSys`**

```rust
    fn rmmod(&self, name: &str) -> Result<(), SysError> {
        // Unlike insmod, a failure here is routine and not an error: the module
        // is usually not loaded yet on a cold boot. The caller logs at debug.
        self.run_to_completion("rmmod", &[name.to_string()])?;
        Ok(())
    }

    fn sleep(&self, d: Duration) {
        std::thread::sleep(d);
    }
```

**Step 3: Verify the mock still builds**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib
```

`#[cfg_attr(test, mockall::automock)]` regenerates `MockSys` automatically.
Expected: everything still passes.

**Step 4: Commit**

```bash
git add cross-compile/anyka-init/src/sys.rs
git commit -m "feat(anyka-init): add rmmod and mockable sleep to the Sys seam"
```

---

### Task A7: `bring_up` orchestration

**Files:**
- Modify: `cross-compile/anyka-init/src/wifi.rs`

This is the impure glue. Keep it thin — every decision it makes should already
be a tested pure function.

**Step 1: Write the failing tests**

These use `MockSys` and a `tempfile::TempDir` for the sysfs and config paths, so
they run on the host with no hardware.

```rust
#[test]
fn test_resolve_chip_prefers_pinned_config_over_hw_conf() {
    // Q4: the shipped config pins the chip, so the detection path is never
    // taken on this camera and hw.conf offset stability stops mattering.
    let c = resolve_chip("ssv6355_ble", "/nonexistent/hw.conf").expect("pinned");
    assert_eq!(c.0.name, "ssv6355_ble");
}

#[test]
fn test_resolve_chip_auto_reads_hw_conf() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("hw.conf");
    std::fs::write(&path, HW_REAL).expect("write");
    let (chip, pol) = resolve_chip("auto", path.to_str().expect("utf8")).expect("auto");
    assert_eq!(chip.name, "ssv6355_ble");
    assert_eq!(pol, Polarity::HighLow);
}

#[test]
fn test_resolve_chip_auto_fails_loudly_on_short_record() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("hw.conf");
    std::fs::write(&path, HW_DEFAULT).expect("write");
    assert!(resolve_chip("auto", path.to_str().expect("utf8")).is_err());
}

#[test]
fn test_resolve_chip_rejects_unknown_pinned_name() {
    assert!(resolve_chip("rtl8189", "/nonexistent").is_err());
}
```

**Step 2: Run, confirm failure**

**Step 3: Implement**

```rust
use crate::config::WifiCfg;
use crate::sys::Sys;

const HW_CONF: &str = "/etc/jffs2/hw.conf";
const HW_CONF_FACTORY: &str = "/mnt/Factory/newFactory/hw.conf";
const GPIO_WIFI_EN: &str = "/sys/user-gpio/wifi_en";
const OTG_MODULE: &str = "/usr/modules/otg-hs.ko";
const WPA_CONF: &str = "/etc/jffs2/wpa_supplicant.conf";
const RESOLV_CONF: &str = "/etc/resolv.conf";
const KO_DIR: &str = "/tmp/ko";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Up {
        chip: &'static str,
        ssid: String,
        addr: String,
    },
    /// Bring-up failed and the vendor chain was invoked instead (R7).
    FellBack,
    /// Bring-up failed and the fallback was disabled by config.
    Failed,
}

/// Resolve the chip and GPIO polarity. Pinned config wins; `auto` parses
/// `hw.conf`, preferring the Factory override when present
/// (`wifi_driver.sh:41-47`).
pub fn resolve_chip(
    pinned: &str,
    hw_conf_path: &str,
) -> Result<(&'static Chip, Polarity), String> {
    if pinned != "auto" {
        let chip = Chip::from_name(pinned)
            .ok_or_else(|| format!("[wifi] chip = {pinned:?} is not a known chip"))?;
        // Polarity comes from config in the pinned case; the caller overrides
        // this with [wifi].gpio_polarity.
        return Ok((chip, Polarity::LowHigh));
    }
    let src = std::fs::read_to_string(hw_conf_path)
        .map_err(|e| format!("cannot read {hw_conf_path}: {e}"))?;
    let hw = parse_hw_conf(&src)
        .ok_or_else(|| format!("{hw_conf_path} is too short or malformed to dispatch a chip"))?;
    let chip = Chip::from_hw_char(hw.chip_char).ok_or_else(|| {
        format!(
            "hw.conf chip character {:?} has no working driver path",
            hw.chip_char
        )
    })?;
    Ok((chip, Polarity::from_char(hw.polarity_char)))
}

/// Full bring-up. Steps are numbered to match the design addendum.
pub fn bring_up(sys: &dyn Sys, cfg: &WifiCfg) -> Outcome {
    match try_bring_up(sys, cfg) {
        Ok(outcome) => outcome,
        Err(e) => {
            tracing::error!(error = %e, "wifi bring-up failed");
            if cfg.fallback_to_vendor {
                fall_back(sys)
            } else {
                tracing::error!(
                    "fallback_to_vendor is disabled; the camera may be unreachable"
                );
                Outcome::Failed
            }
        }
    }
}

/// R7. Regenerates the vendor-shaped conf before delegating, because the
/// RTL8188 path `sed`s into line numbers 3 and 4 (R10).
fn fall_back(sys: &dyn Sys) -> Outcome {
    tracing::error!("falling back to the vendor wifi chain: /usr/sbin/wifi_manage.sh start");
    match sys.run_to_completion("/usr/sbin/wifi_manage.sh", &["start".to_string()]) {
        Ok(st) => tracing::warn!(?st, "vendor wifi_manage.sh start invoked"),
        Err(e) => tracing::error!(error = %e, "vendor fallback also failed"),
    }
    Outcome::FellBack
}
```

Then the body. Write it step by step, matching the design's numbered sequence.
Each step logs before acting so a hardware hang is diagnosable from the log:

```rust
fn try_bring_up(sys: &dyn Sys, cfg: &WifiCfg) -> Result<Outcome, String> {
    // 1. Resolve chip.
    let hw_path = if std::path::Path::new(HW_CONF_FACTORY).exists() {
        HW_CONF_FACTORY
    } else {
        HW_CONF
    };
    let (chip, detected_pol) = resolve_chip(&cfg.chip, hw_path)?;
    let polarity = if cfg.chip == "auto" {
        detected_pol
    } else {
        Polarity::from_name(&cfg.gpio_polarity)
            .ok_or_else(|| format!("[wifi] gpio_polarity = {:?}", cfg.gpio_polarity))?
    };
    tracing::info!(chip = chip.name, ?polarity, "wifi chip resolved");

    // 2. Validate credentials before touching hardware.
    let sec = Security::from_name(&cfg.security)
        .ok_or_else(|| format!("[wifi] security = {:?}", cfg.security))?;
    validate_credentials(&cfg.ssid, &cfg.password, sec)?;

    // 3. Power sequence (wifi_driver.sh:373-382).
    for level in polarity.sequence() {
        std::fs::write(GPIO_WIFI_EN, level)
            .map_err(|e| format!("gpio write {GPIO_WIFI_EN}={level}: {e}"))?;
        sys.sleep(Duration::from_secs(1));
    }

    // 4. Prepare (wifi_driver.sh:104-112, 197).
    std::fs::create_dir_all(KO_DIR).map_err(|e| format!("mkdir {KO_DIR}: {e}"))?;
    let _ = sys.run_to_completion(
        "tar",
        &["zxf".into(), "/data/wifi_driver.tgz".into(), "-C".into(), KO_DIR.into()],
    );
    let _ = sys.run_to_completion(
        "tar",
        &["zxf".into(), "/data/wifi_tool.tgz".into(), "-C".into(), "/tmp".into()],
    );
    let _ = sys.insmod(OTG_MODULE);
    // R9: the vendor papers over USB enumeration timing with `sleep 3`. Keep it
    // until hardware confirms what it is actually waiting for.
    sys.sleep(Duration::from_secs(3));

    // 5. Load driver.
    let _ = sys.rmmod(chip.rmmod); // routine failure on a cold boot
    sys.run_to_completion("insmod", &chip.insmod_args())
        .map_err(|e| format!("insmod {}: {e}", chip.module))?;
    if !chip.settle.is_zero() {
        sys.sleep(chip.settle);
    }

    // 6. Wait for the interface (wifi_run.sh:76-86, 30 s cap).
    wait_for_interface(sys, &cfg.interface, Duration::from_secs(30))?;
    sys.run_to_completion("ifconfig", &[cfg.interface.clone(), "up".into()])
        .map_err(|e| format!("ifconfig up: {e}"))?;

    // 7. Write wpa_supplicant.conf.
    std::fs::write(WPA_CONF, wpa_supplicant_conf(&cfg.ssid, &cfg.password, sec))
        .map_err(|e| format!("write {WPA_CONF}: {e}"))?;

    // 8-9. wpa_supplicant is a supervised service started in P3 (design Q1), so
    // bring-up starts it once here in the foreground-detached form and waits for
    // association. See Task A8 for the driver-flag probe (R8).
    let driver = start_supplicant_probing_driver(sys, cfg)?;
    tracing::info!(driver, "wpa_supplicant associated");

    // 10-12: address, resolv.conf, verification. See Task A8.
    let addr = assign_address(sys, cfg)?;
    if !cfg.dns.is_empty() {
        std::fs::write(RESOLV_CONF, resolv_conf(&cfg.dns))
            .map_err(|e| format!("write {RESOLV_CONF}: {e}"))?;
    }

    Ok(Outcome::Up {
        chip: chip.name,
        ssid: cfg.ssid.clone(),
        addr,
    })
}

fn wait_for_interface(sys: &dyn Sys, iface: &str, cap: Duration) -> Result<(), String> {
    let path = format!("/sys/class/net/{iface}");
    let deadline = sys.now() + cap;
    while sys.now() < deadline {
        if std::path::Path::new(&path).exists() {
            return Ok(());
        }
        sys.sleep(Duration::from_secs(1));
    }
    Err(format!("{iface} did not appear within {cap:?}"))
}
```

**Step 4: Run, confirm the `resolve_chip` tests pass**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib test_resolve_chip
```

Expected: 4 passed.

**Step 5: Commit**

```bash
git add cross-compile/anyka-init/src/wifi.rs
git commit -m "feat(anyka-init): wifi bring-up orchestration with mandatory vendor fallback"
```

---

### Task A8: Association, addressing, and the driver probe

**Files:**
- Modify: `cross-compile/anyka-init/src/wifi.rs`

**Step 1: Write the failing tests**

```rust
#[test]
fn test_supplicant_args_uses_probed_driver_and_no_dash_b() {
    // Q1: -B self-daemonizes, which the supervisor would read as an instant
    // crash and backoff-loop on forever. wifi_station.sh:60 already proves
    // foreground operation works on this hardware.
    let args = supplicant_args("wlan0", "nl80211");
    assert!(!args.contains(&"-B".to_string()), "must not self-daemonize");
    assert_eq!(args, vec!["-i", "wlan0", "-D", "nl80211", "-c", WPA_CONF]);
}

#[test]
fn test_driver_probe_order_is_nl80211_then_wext() {
    // R8: RTL8188 with wpa_supplicant 2.6 needs nl80211; everything else uses
    // wext (wifi_station.sh:55-68).
    assert_eq!(DRIVER_PROBE_ORDER, ["nl80211", "wext"]);
}

#[test]
fn test_static_address_argv_is_wellformed() {
    let cidr = parse_cidr("192.168.2.198/24").expect("cidr");
    let args = ifconfig_static_args("wlan0", &cidr);
    assert_eq!(
        args,
        vec!["wlan0", "192.168.2.198", "netmask", "255.255.255.0"]
    );
}

#[test]
fn test_udhcpc_oneshot_args_exit_after_lease() {
    // -n exits if no lease, -q quits once one is obtained. Without both, the
    // one-shot never returns and P2 blocks forever.
    let args = udhcpc_oneshot_args("wlan0");
    assert!(args.contains(&"-n".to_string()));
    assert!(args.contains(&"-q".to_string()));
    assert_eq!(args[0], "udhcpc", "busybox multicall: argv[0] selects the applet");
}
```

**Step 2: Run, confirm failure**

**Step 3: Implement**

```rust
pub const DRIVER_PROBE_ORDER: [&str; 2] = ["nl80211", "wext"];
const BUSYBOX: &str = "/bin/busybox";

pub fn supplicant_args(iface: &str, driver: &str) -> Vec<String> {
    vec![
        "-i".into(),
        iface.into(),
        "-D".into(),
        driver.into(),
        "-c".into(),
        WPA_CONF.into(),
    ]
}

pub fn ifconfig_static_args(iface: &str, cidr: &Cidr) -> Vec<String> {
    vec![
        iface.into(),
        cidr.address.clone(),
        "netmask".into(),
        cidr.netmask.clone(),
    ]
}

/// Busybox multicall form: `/sbin/udhcpc` does not exist on this device
/// (`/sbin` holds only ldconfig, mmc_test, updater) and the `orig/` capture
/// lost its symlinks, so `argv[0]` selects the applet instead (R17).
pub fn udhcpc_oneshot_args(iface: &str) -> Vec<String> {
    vec![
        "udhcpc".into(),
        "-i".into(),
        iface.into(),
        "-n".into(),
        "-q".into(),
    ]
}
```

Then `start_supplicant_probing_driver` and `assign_address`:

```rust
fn start_supplicant_probing_driver(sys: &dyn Sys, cfg: &WifiCfg) -> Result<&'static str, String> {
    let timeout = Duration::from_secs(cfg.connect_timeout_sec);
    for driver in DRIVER_PROBE_ORDER {
        tracing::info!(driver, "starting wpa_supplicant");
        let _ = sys.run_to_completion("killall", &["wpa_supplicant".into()]);
        // Spawned detached here only for the duration of bring-up. P3 registers
        // it as a supervised service with whichever driver flag worked.
        sys.spawn_detached("/usr/sbin/wpa_supplicant", &supplicant_args(&cfg.interface, driver))
            .map_err(|e| format!("spawn wpa_supplicant: {e}"))?;
        if wait_associated(sys, &cfg.interface, timeout) {
            return Ok(driver);
        }
        tracing::warn!(driver, "no association; trying the next driver flag");
    }
    Err("no driver flag produced an association".into())
}

fn wait_associated(sys: &dyn Sys, iface: &str, cap: Duration) -> bool {
    let deadline = sys.now() + cap;
    while sys.now() < deadline {
        if read_carrier(iface) == Some(true) {
            return true;
        }
        sys.sleep(Duration::from_secs(1));
    }
    false
}

fn assign_address(sys: &dyn Sys, cfg: &WifiCfg) -> Result<String, String> {
    if cfg.dhcp {
        return dhcp_once(sys, cfg);
    }
    let cidr = parse_cidr(cfg.address.as_deref().unwrap_or(""))
        .ok_or_else(|| format!("[wifi] address = {:?} is not valid CIDR", cfg.address))?;
    let gw = cfg.gateway.clone().unwrap_or_default();
    sys.run_to_completion("ifconfig", &ifconfig_static_args(&cfg.interface, &cidr))
        .map_err(|e| format!("ifconfig static: {e}"))?;
    sys.run_to_completion(
        "route",
        &["add".into(), "default".into(), "gw".into(), gw.clone()],
    )
    .map_err(|e| format!("route add default: {e}"))?;

    // R12: a typo'd static address associates fine and leaves the camera
    // unreachable, which no rung of R7 would catch. Verify, then fall back to
    // DHCP once before giving up.
    if gateway_reachable(sys, &gw) {
        return Ok(cidr.address);
    }
    tracing::error!(gateway = %gw, "static address assigned but gateway unreachable; retrying via DHCP");
    dhcp_once(sys, cfg)
}

fn dhcp_once(sys: &dyn Sys, cfg: &WifiCfg) -> Result<String, String> {
    sys.run_to_completion(BUSYBOX, &udhcpc_oneshot_args(&cfg.interface))
        .map_err(|e| format!("udhcpc: {e}"))?;
    read_address(&cfg.interface).ok_or_else(|| "no address after udhcpc".into())
}
```

`read_carrier`, `read_address` and `gateway_reachable` are implemented in
Phase B (Task B1–B3) and shared. For Phase A, implement `read_carrier` and
`read_address` now as thin `std::fs` readers over
`/sys/class/net/<if>/carrier` and `/proc/net/route`, and make
`gateway_reachable` a stub returning `true` with a `ponytail:` comment naming
Task B3 as the upgrade path. Phase A must not block on Phase B.

Add `spawn_detached` to the `Sys` trait alongside `spawn`: same `Command`
construction, but without the log-file plumbing, since bring-up runs before the
logging service plumbing is set up.

**Step 4: Run, confirm pass** — 4 passed.

**Step 5: Commit**

```bash
git add cross-compile/anyka-init/src/wifi.rs cross-compile/anyka-init/src/sys.rs
git commit -m "feat(anyka-init): association, driver-flag probe, and address assignment"
```

---

### Task A9: Wire into P2 and ship the config

**Files:**
- Modify: `cross-compile/anyka-init/src/boot.rs:98-107`
- Modify: `SD_card_contents/anyka_hack/anyka.toml`
- Modify: `docs/plans/2026-08-01-boot-runtime-rust-design.md` (status line only)

**Step 1: Replace the vendor call in `system_setup`**

`src/boot.rs:104-107` currently calls `wifi_manage.sh start` unconditionally.
Replace it — but keep `apply_wifi` above it, because `anyka_ipc` and the WebUI
still read `anyka_cfg.ini` and the R7 fallback needs it populated (W5).

```rust
    match wifi::bring_up(sys, &cfg.wifi) {
        wifi::Outcome::Up { chip, ref ssid, ref addr } => {
            tracing::info!(chip, ssid, addr, "wifi up");
        }
        wifi::Outcome::FellBack => {
            tracing::error!("wifi came up via the vendor fallback; check the chip dispatch");
        }
        wifi::Outcome::Failed => {
            tracing::error!("wifi is down and the fallback is disabled");
        }
    }
```

**Step 2: Update the shipped config**

Add to `SD_card_contents/anyka_hack/anyka.toml` under `[wifi]`, and add the two
new service blocks. Copy the exact block from the design addendum's *Config
additions* section. Keep `ssid`/`password` as `CHANGE_ME`.

**Step 3: Run the whole suite**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib
$CARGO test --target x86_64-unknown-linux-gnu --test supervision -- --test-threads=1
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO fmt --check
```

Expected: all pass, no warnings.

**Step 4: Cross-compile**

```bash
cd "$(git rev-parse --show-toplevel)"
./scripts/build_sd_contents.sh
```

Expected: `anyka-init installed to .../anyka_hack/anyka-init.bin`. The target is
`armv5te-unknown-linux-uclibceabi`.

**Step 5: Commit**

```bash
git add cross-compile/anyka-init/src/boot.rs SD_card_contents/anyka_hack/anyka.toml
git commit -m "feat(anyka-init): replace wifi_manage.sh with wifi::bring_up in P2"
```

---

### Task A10: Hardware smoke test

**STOP. Do not proceed to Phase B until this passes.** W6: wifi is the only
component in this design whose failure removes its own recovery path.

Before booting, confirm you can reach the camera another way. Keep a copy of the
previous working SD card.

| # | Check | Expected |
|---|---|---|
| 1 | Boot with `chip = "ssv6355_ble"` and correct credentials | `wifi up` in the log, camera pingable |
| 2 | `ls -l /etc/resolv.conf` on the device | Answers the open W7 question; the file we write should be there |
| 3 | `ls -l /bin/busybox && /bin/busybox udhcpc --help` | Confirms R17 |
| 4 | `logread`/log file for `Outcome::FellBack` | Must be absent on a good boot |
| 5 | Boot with `chip = "auto"` | Same result as check 1 — confirms hw.conf offset 51 on this board |
| 6 | Boot with a deliberately wrong password | Loud `4-Way Handshake failed` in the wpa_supplicant log, then the R7 fallback |
| 7 | Boot with `dhcp = false` and a correct static address | Camera reachable at the configured address |
| 8 | Boot with `dhcp = false` and a *wrong* gateway | R12 retry: log shows the DHCP retry and the camera is still reachable |
| 9 | Pull the SD card, reboot | Camera returns to stock vendor behaviour |

Record the results in the design doc's *Open questions* section, replacing the
two hardware-confirmation items.

---

# Phase B — Link Monitoring

Independent of Phase A. Touches no boot-path code. Ships separately.

---

### Task B1: Read link state

**Files:**
- Create: `cross-compile/anyka-init/src/netstat.rs`
- Modify: `cross-compile/anyka-init/src/lib.rs`

Pure parsers first; the monitor wires them up in Task B4.

**Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Real format: `cat /proc/net/route` on a busybox system. Tabs, not spaces.
    const ROUTE: &str = "\
Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT
wlan0\t00000000\t0102A8C0\t0003\t0\t0\t0\t00000000\t0\t0\t0
wlan0\t0002A8C0\t00000000\t0001\t0\t0\t0\t00FFFFFF\t0\t0\t0
";

    #[test]
    fn test_parse_default_route_decodes_little_endian_hex_gateway() {
        // 0102A8C0 is 192.168.2.1 stored little-endian.
        let gw = parse_default_route(ROUTE, "wlan0").expect("default route present");
        assert_eq!(gw, "192.168.2.1");
    }

    #[test]
    fn test_parse_default_route_ignores_other_interfaces() {
        assert!(parse_default_route(ROUTE, "eth0").is_none());
    }

    #[test]
    fn test_parse_default_route_none_when_only_subnet_routes() {
        let only_subnet = "Iface\tDestination\tGateway\tFlags\n\
                           wlan0\t0002A8C0\t00000000\t0001\t0\t0\t0\t00FFFFFF\t0\t0\t0\n";
        assert!(parse_default_route(only_subnet, "wlan0").is_none());
    }

    #[test]
    fn test_parse_default_route_handles_empty_and_header_only() {
        assert!(parse_default_route("", "wlan0").is_none());
        assert!(parse_default_route("Iface\tDestination\tGateway\n", "wlan0").is_none());
    }

    #[test]
    fn test_parse_operstate_only_up_is_up() {
        assert!(parse_operstate("up\n"));
        assert!(!parse_operstate("down\n"));
        assert!(!parse_operstate("dormant\n"));
        assert!(!parse_operstate("unknown\n"));
        assert!(!parse_operstate(""));
    }

    const ARP: &str = "\
IP address       HW type     Flags       HW address            Mask     Device
192.168.2.1      0x1         0x2         a4:2b:b0:11:22:33     *        wlan0
192.168.2.50     0x1         0x0         00:00:00:00:00:00     *        wlan0
";

    #[test]
    fn test_parse_arp_complete_entry_is_reachable() {
        // Flags 0x2 is ATF_COM: the entry is resolved.
        assert!(arp_entry_complete(ARP, "192.168.2.1"));
    }

    #[test]
    fn test_parse_arp_incomplete_entry_is_not_reachable() {
        assert!(!arp_entry_complete(ARP, "192.168.2.50"));
    }

    #[test]
    fn test_parse_arp_absent_entry_is_not_reachable() {
        assert!(!arp_entry_complete(ARP, "192.168.2.99"));
        assert!(!arp_entry_complete("", "192.168.2.1"));
    }
}
```

**Step 2: Run, confirm failure**

**Step 3: Implement**

```rust
//! Link-state readers for the wifi monitor.
//!
//! Every function here is a pure parse over a `/proc` or `/sys` file's
//! contents, so the whole health check is provable on the host.

use std::net::Ipv4Addr;

/// `/sys/class/net/<if>/operstate`. Only `up` counts; `dormant` means
/// associated-but-not-ready and `unknown` is what a driver reports when it
/// does not implement the callback.
pub fn parse_operstate(src: &str) -> bool {
    src.trim() == "up"
}

/// Extract the default gateway for `iface` from `/proc/net/route`.
///
/// Read from the kernel rather than from `[wifi].gateway` on purpose: it works
/// identically for DHCP and static, and the monitor can never probe an address
/// the kernel is not actually routing through.
pub fn parse_default_route(src: &str, iface: &str) -> Option<String> {
    for line in src.lines().skip(1) {
        let mut f = line.split_whitespace();
        let (Some(dev), Some(dest), Some(gw)) = (f.next(), f.next(), f.next()) else {
            continue;
        };
        if dev != iface || dest != "00000000" {
            continue;
        }
        let raw = u32::from_str_radix(gw, 16).ok()?;
        if raw == 0 {
            continue;
        }
        // /proc/net/route stores addresses little-endian.
        return Some(Ipv4Addr::from(raw.swap_bytes()).to_string());
    }
    None
}

/// ATF_COM: the ARP entry is resolved, so the gateway answered at L2.
const ATF_COM: u32 = 0x2;

/// Does `/proc/net/arp` hold a complete entry for `addr`?
pub fn arp_entry_complete(src: &str, addr: &str) -> bool {
    for line in src.lines().skip(1) {
        let mut f = line.split_whitespace();
        let (Some(ip), Some(_hw_type), Some(flags)) = (f.next(), f.next(), f.next()) else {
            continue;
        };
        if ip != addr {
            continue;
        }
        let flags = flags
            .strip_prefix("0x")
            .and_then(|h| u32::from_str_radix(h, 16).ok())
            .unwrap_or(0);
        return flags & ATF_COM != 0;
    }
    false
}
```

**Step 4: Run, confirm pass** — 8 passed.

**Step 5: Commit**

```bash
git add cross-compile/anyka-init/src/netstat.rs cross-compile/anyka-init/src/lib.rs
git commit -m "feat(anyka-init): pure link-state parsers for operstate, route and arp"
```

---

### Task B2: The active probe

**Files:**
- Modify: `cross-compile/anyka-init/src/netstat.rs`

An L2 probe, not TCP. `TcpStream::connect_timeout(gw:80)` reads a RST as alive,
but a router that silently drops on a closed port reads as dead. Forcing ARP
resolution and checking for a complete entry is immune to firewall policy.

**Step 1: Implement**

```rust
use std::net::UdpSocket;
use std::time::Duration;

/// Poke the gateway so the kernel resolves it, then read the ARP table.
///
/// Port 9 is `discard`. Nothing needs to be listening — the UDP send is only
/// there to force address resolution. A `Result` from `send_to` is ignored for
/// the same reason.
pub fn gateway_reachable(gw: &str) -> bool {
    if let Ok(sock) = UdpSocket::bind("0.0.0.0:0") {
        let _ = sock.set_write_timeout(Some(Duration::from_millis(200)));
        let _ = sock.send_to(&[0u8; 1], format!("{gw}:9"));
    }
    std::thread::sleep(Duration::from_millis(200));
    std::fs::read_to_string("/proc/net/arp")
        .map(|src| arp_entry_complete(&src, gw))
        .unwrap_or(false)
}
```

**Step 2: Replace the Phase A stub**

`wifi.rs`'s `gateway_reachable` stub from Task A8 becomes a call to this. Delete
the `ponytail:` comment naming Task B3.

**Step 3: Run the suite, commit**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib
git add cross-compile/anyka-init/src/netstat.rs cross-compile/anyka-init/src/wifi.rs
git commit -m "feat(anyka-init): L2 gateway reachability probe via ARP"
```

---

### Task B3: The escalation policy

**Files:**
- Modify: `cross-compile/anyka-init/src/netstat.rs`

The whole recovery policy as one pure function, mirroring the existing
`supervise::decide` seam.

**Step 1: Write the failing tests**

```rust
#[test]
fn test_decide_healthy_link_does_nothing() {
    let h = Health { carrier: true, route: true, reachable: true };
    assert_eq!(decide(h, 0, &POLICY), Action::Nothing);
    assert_eq!(decide(h, 99, &POLICY), Action::Nothing);
}

#[test]
fn test_decide_waits_before_acting() {
    // A single bad tick is not enough. R16: this also absorbs one stale ARP
    // read.
    let h = Health { carrier: true, route: false, reachable: false };
    assert_eq!(decide(h, 1, &POLICY), Action::Nothing);
    assert_eq!(decide(h, 2, &POLICY), Action::Nothing);
    assert_eq!(decide(h, 3, &POLICY), Action::RunDhcp);
}

#[test]
fn test_decide_missing_route_runs_dhcp() {
    let h = Health { carrier: true, route: false, reachable: false };
    assert_eq!(decide(h, 3, &POLICY), Action::RunDhcp);
}

#[test]
fn test_decide_no_carrier_restarts_supplicant() {
    let h = Health { carrier: false, route: false, reachable: false };
    assert_eq!(decide(h, 5, &POLICY), Action::RestartSupplicant);
}

#[test]
fn test_decide_l3_blackhole_restarts_supplicant() {
    // Associated and addressed but nothing answers: the case operstate alone
    // reports as perfectly healthy.
    let h = Health { carrier: true, route: true, reachable: false };
    assert_eq!(decide(h, 5, &POLICY), Action::RestartSupplicant);
}

#[test]
fn test_decide_escalates_to_reboot_after_the_long_threshold() {
    let h = Health { carrier: false, route: false, reachable: false };
    assert_eq!(decide(h, 10, &POLICY), Action::Reboot);
}

#[test]
fn test_decide_stops_escalating_past_the_reboot_cap() {
    // R14: an AP that is off overnight must not produce an unbounded reboot
    // loop.
    let h = Health { carrier: false, route: false, reachable: false };
    let exhausted = Policy { wifi_reboots_used: 3, ..POLICY };
    assert_eq!(decide(h, 10, &exhausted), Action::LogOnly);
    assert_eq!(decide(h, 1000, &exhausted), Action::LogOnly);
}
```

Define `POLICY` in the tests module with the shipped defaults (3 / 5 / 10, cap 3).

**Step 2: Run, confirm failure**

**Step 3: Implement**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Health {
    pub carrier: bool,
    pub route: bool,
    /// Only meaningful when `carrier && route`; the caller does not probe
    /// otherwise.
    pub reachable: bool,
}

impl Health {
    pub fn ok(&self) -> bool {
        self.carrier && self.route && self.reachable
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    pub dhcp_after_ticks: u32,
    pub supplicant_after_ticks: u32,
    pub reboot_after_ticks: u32,
    pub reboot_cap: u8,
    /// Persisted across reboots; zeroed only by a successful `bring_up`.
    pub wifi_reboots_used: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Nothing,
    RunDhcp,
    RestartSupplicant,
    Reboot,
    /// Escalation exhausted. Keep logging; keep serving video locally.
    LogOnly,
}

/// The whole recovery ladder. `ticks` counts *consecutive* unhealthy samples
/// and is reset by the caller after any action, so the next escalation starts
/// one rung higher.
pub fn decide(h: Health, ticks: u32, p: &Policy) -> Action {
    if h.ok() {
        return Action::Nothing;
    }
    if ticks >= p.reboot_after_ticks {
        return if p.wifi_reboots_used < p.reboot_cap {
            Action::Reboot
        } else {
            Action::LogOnly
        };
    }
    if !h.route && ticks >= p.dhcp_after_ticks {
        return Action::RunDhcp;
    }
    if ticks >= p.supplicant_after_ticks {
        return Action::RestartSupplicant;
    }
    Action::Nothing
}
```

**Step 4: Run, confirm pass** — 7 passed.

**Step 5: Commit**

```bash
git add cross-compile/anyka-init/src/netstat.rs
git commit -m "feat(anyka-init): pure escalation policy for wifi link recovery"
```

---

### Task B4: Persist the wifi reboot counter

**Files:**
- Modify: `cross-compile/anyka-init/src/storm.rs:17-45`

`monitor.rs:38` zeroes `fast_reboots` once uptime passes the threshold. That is
right for a boot-time crash loop and wrong for a runtime trigger: a
wifi-triggered reboot always fires after the reset, so the guard would be back
at zero every time. A once-per-boot latch does not fix it either — reboot, come
up, find wifi still down, wait ten minutes, reboot again.

The counter must be reset by evidence the problem is gone.

**Step 1: Write the failing tests**

```rust
#[test]
fn test_storm_state_round_trips_both_counters() {
    let s = StormState { fast_reboots: 2, wifi_reboots: 1 };
    assert_eq!(StormState::parse(&s.render()), s);
}

#[test]
fn test_storm_state_reads_legacy_single_field_file() {
    // Files written before this change must not read as a torn file and reset
    // the crash-loop guard.
    let legacy = r#"{"fast_reboots":2}"#;
    let s = StormState::parse(legacy);
    assert_eq!(s.fast_reboots, 2);
    assert_eq!(s.wifi_reboots, 0);
}

#[test]
fn test_storm_state_torn_file_reads_as_zero() {
    for bad in [r#"{"fast_reboots":"#, "", "garbage", r#"{"fast_reboots":250}"#] {
        assert_eq!(StormState::parse(bad), StormState::default());
    }
}
```

**Step 2: Implement**

Add `pub wifi_reboots: u8` to the struct. Replace the hand-rolled
`strip_prefix`/`strip_suffix` parse with a small field reader so two fields do
not double the code:

```rust
    fn field(src: &str, name: &str) -> Option<u8> {
        let key = format!("\"{name}\":");
        let rest = src.split_once(&key)?.1;
        let end = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        match rest[..end].parse::<u8>() {
            Ok(n) if n <= MAX_SANE_REBOOTS => Some(n),
            _ => None,
        }
    }

    pub fn parse(src: &str) -> Self {
        let src = src.trim();
        if !src.starts_with('{') || !src.ends_with('}') {
            return Self::default();
        }
        Self {
            fast_reboots: Self::field(src, "fast_reboots").unwrap_or(0),
            wifi_reboots: Self::field(src, "wifi_reboots").unwrap_or(0),
        }
    }

    pub fn render(&self) -> String {
        format!(
            r#"{{"fast_reboots":{},"wifi_reboots":{}}}"#,
            self.fast_reboots, self.wifi_reboots
        )
    }
```

**Step 3: Reset it on success**

In `wifi::bring_up`, on `Outcome::Up`, load the storm state, set
`wifi_reboots = 0`, save. This is the only place it is zeroed. Do not reset it
on uptime.

**Step 4: Run, confirm pass, commit**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib
git add cross-compile/anyka-init/src/storm.rs cross-compile/anyka-init/src/wifi.rs
git commit -m "fix(anyka-init): persist a wifi reboot counter reset only by a successful bring-up"
```

---

### Task B5: `Msg::RestartService`

**Files:**
- Modify: `cross-compile/anyka-init/src/supervisor_loop.rs:32-35` and `:103-210`
- Test: `cross-compile/anyka-init/tests/supervision.rs`

A monitor thread calling `kill()` itself would race the supervisor's backoff
timer for the same child. Route the request through the channel that already
exists, so the supervisor stays the sole owner of process state (R15).

**Step 1: Write the failing integration test**

Add to `tests/supervision.rs`, following the existing helpers there:

```rust
#[test]
fn test_restart_service_message_respawns_the_child() {
    // A long-running service that would never exit on its own.
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg_path = write_config(dir.path(), &sleeper_service(dir.path()));
    // ... spawn the supervisor on a thread, wait for the first pid,
    // send Msg::RestartService("sleeper".into()), assert a *different* pid
    // appears within the backoff window.
}
```

Model it on whichever existing test in that file waits for a respawn. Keep the
`--test-threads=1` requirement in mind: these call `waitpid(-1)`.

**Step 2: Add the variant**

```rust
pub enum Msg {
    Exited(Pid, ExitStatus),
    Shutdown,
    /// Recovery request from the monitor thread. The supervisor kills the named
    /// service; the normal exit path then restarts it under the usual backoff.
    RestartService(String),
}
```

**Step 3: Handle it in `run`**

Next to the existing `Msg::Exited` and `Msg::Shutdown` arms:

```rust
                Ok(Msg::RestartService(name)) => {
                    match services.iter().find(|s| s.name == name) {
                        Some(svc) => match svc.state.pid() {
                            Some(pid) => {
                                tracing::warn!(service = %name, pid, "restart requested by monitor");
                                let _ = sys.kill(pid, libc::SIGTERM);
                            }
                            None => tracing::info!(
                                service = %name,
                                "restart requested but the service is not running"
                            ),
                        },
                        None => tracing::warn!(service = %name, "restart requested for unknown service"),
                    }
                }
```

Killing rather than respawning directly is deliberate: the existing `Exited`
path already owns backoff, crash-loop counting and the storm guard. Adding a
second respawn path would duplicate all of it.

**Step 4: Run, confirm pass**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --test supervision -- --test-threads=1
```

**Step 5: Commit**

```bash
git add cross-compile/anyka-init/src/supervisor_loop.rs cross-compile/anyka-init/tests/supervision.rs
git commit -m "feat(anyka-init): monitor-driven service restart via the supervisor channel"
```

---

### Task B6: Wire the monitor

**Files:**
- Modify: `cross-compile/anyka-init/src/monitor.rs:22-47`
- Modify: `cross-compile/anyka-init/src/config.rs` (`MonitorCfg`)
- Modify: `SD_card_contents/anyka_hack/anyka.toml`

**Step 1: Extend `MonitorCfg`**

```rust
    #[serde(default = "d_true")]
    pub wifi: bool,
    #[serde(default = "d_true")]
    pub wifi_probe: bool,
    #[serde(default = "d_wifi_dhcp_ticks")]
    pub wifi_dhcp_after_ticks: u32,
    #[serde(default = "d_wifi_supplicant_ticks")]
    pub wifi_supplicant_after_ticks: u32,
    #[serde(default = "d_wifi_reboot_ticks")]
    pub wifi_reboot_after_ticks: u32,
    #[serde(default = "d_wifi_reboot_cap")]
    pub wifi_reboot_cap: u8,
```

Defaults 3 / 5 / 10 / 3. At the 60-second tick that is 3, 5 and 10 minutes.

**Step 2: Sample and act**

`monitor::run` gains the `Sender<Msg>` and the wifi config. In the loop, after
the existing `sample()`:

```rust
        if cfg.wifi {
            let h = sample_link(&iface, cfg.wifi_probe);
            if h.ok() {
                ticks = 0;
            } else {
                ticks += 1;
                tracing::warn!(
                    carrier = h.carrier,
                    route = h.route,
                    reachable = h.reachable,
                    ticks,
                    "wifi link unhealthy"
                );
            }
            let policy = Policy { wifi_reboots_used: storm.wifi_reboots, ..base_policy };
            match netstat::decide(h, ticks, &policy) {
                Action::Nothing => {}
                Action::RunDhcp => {
                    tracing::warn!("no default route; re-running udhcpc");
                    let _ = sys.run_to_completion(BUSYBOX, &udhcpc_oneshot_args(&iface));
                    ticks = 0;
                }
                Action::RestartSupplicant => {
                    let _ = tx.send(Msg::RestartService("wpa_supplicant".into()));
                    ticks = 0;
                }
                Action::Reboot => {
                    tracing::error!(
                        wifi_reboots = storm.wifi_reboots,
                        "wifi down past the reboot threshold; rebooting"
                    );
                    storm.wifi_reboots += 1;
                    let _ = storm.save(state_path);
                    let _ = sys.reboot();
                }
                Action::LogOnly => {
                    tracing::error!("wifi down and the reboot budget is exhausted; not rebooting");
                }
            }
        }
```

**Step 3: Add the config block to the shipped `anyka.toml`**

Copy from the design addendum's monitoring section.

**Step 4: Full verification**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib
$CARGO test --target x86_64-unknown-linux-gnu --test supervision -- --test-threads=1
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
$CARGO fmt --check
cd "$(git rev-parse --show-toplevel)" && ./scripts/build_sd_contents.sh
```

**Step 5: Commit**

```bash
git add cross-compile/anyka-init/src/monitor.rs cross-compile/anyka-init/src/config.rs \
        SD_card_contents/anyka_hack/anyka.toml
git commit -m "feat(anyka-init): wifi link monitoring with bounded escalation"
```

---

### Task B7: Hardware validation

| # | Check | How | Expected |
|---|---|---|---|
| 1 | Healthy link is quiet | Boot, watch the log for 10 min | No `wifi link unhealthy` lines |
| 2 | Route loss triggers DHCP | `route del default` on the device | `re-running udhcpc` within 3 min, route restored |
| 3 | Association loss restarts the supplicant | `killall -STOP wpa_supplicant` | `restart requested by monitor` within 5 min |
| 4 | Reboot rung is bounded | Power off the AP, leave the camera 45 min | At most 3 reboots, then `reboot budget is exhausted` |
| 5 | Counter resets on recovery | Power the AP back on | `wifi up`, and `wifi_reboots` back to 0 in `boot.json` |
| 6 | Probe does not false-positive | Watch a full night on a healthy link | No spurious actions |

Check 4 is the important one. It is the failure mode that R14 exists to prevent,
and the only way to confirm it is to actually leave the AP off.

---

## Done When

- Phase A: the camera associates and gets an address with no vendor wifi script
  in the process tree, and A10 checks 1–9 pass.
- Phase B: B7 checks 1–6 pass, in particular check 4.
- `$CARGO clippy -- -D warnings` and `$CARGO fmt --check` clean.
- The design doc's *Open questions* section is updated with the W7 and R17
  hardware answers.
