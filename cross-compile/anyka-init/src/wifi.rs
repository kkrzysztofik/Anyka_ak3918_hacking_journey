//! Wifi bring-up, replacing the vendor chain: wifi_manage.sh -> wifi_run.sh ->
//! wifi_driver.sh / wifi_station.sh -> station_connect.sh (1,316 lines of sh).
//!
//! See docs/plans/2026-08-01-boot-runtime-rust-design.md, addendum.

use std::net::Ipv4Addr;
use std::time::Duration;

use crate::config::WifiCfg;
use crate::sys::Sys;

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
        if c == '2' {
            Self::HighLow
        } else {
            Self::LowHigh
        }
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
        return Err(format!(
            "[wifi] password contains unsupported character {c:?}"
        ));
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
    // Probe actively instead of waiting to hear a beacon. Required for an SSID
    // that does not broadcast, and materially faster to acquire at the edge of
    // range: a directed probe request draws a probe response, where passive
    // scanning has to catch a beacon that may not survive the path loss. .121
    // roams between a router at -49 dBm and a mesh satellite at -85 dBm, and on
    // the satellite it repeatedly failed to associate inside connect_timeout.
    s.push_str("\tscan_ssid=1\n");
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

const HW_CONF: &str = "/etc/jffs2/hw.conf";
const HW_CONF_FACTORY: &str = "/mnt/Factory/newFactory/hw.conf";
const GPIO_WIFI_EN: &str = "/sys/user-gpio/wifi_en";
const OTG_MODULE: &str = "/usr/modules/otg-hs.ko";
const WPA_CONF: &str = "/etc/jffs2/wpa_supplicant.conf";
const RESOLV_CONF: &str = "/etc/resolv.conf";
const KO_DIR: &str = "/tmp/ko";
/// Same class of bug as the `/sbin/udhcpc` one (R17): `/usr/sbin/wpa_supplicant`
/// does not exist on this camera. The rootfs ships `/usr/bin/wpa_supplicant` as
/// a symlink into `/tmp`, and the `orig/` capture lost every symlink, so the
/// path could not be checked from the capture alone. The extraction target is
/// the one path that is real on any revision: step 4 unpacks
/// `/data/wifi_tool.tgz` into `/tmp`, which is where the binary comes from.
const SUPPLICANT_BIN: &str = "/tmp/wpa_supplicant";
pub const DRIVER_PROBE_ORDER: [&str; 2] = ["nl80211", "wext"];
const BUSYBOX: &str = "/bin/busybox";
const SYS_CLASS_NET: &str = "/sys/class/net";
const PROC_FIB_TRIE: &str = "/proc/net/fib_trie";
const PROC_ROUTE: &str = "/proc/net/route";
const WIFI_MANAGE_SCRIPT: &str = "/usr/sbin/wifi_manage.sh";
const WIFI_DRIVER_TGZ: &str = "/data/wifi_driver.tgz";
const WIFI_TOOL_TGZ: &str = "/data/wifi_tool.tgz";

/// Every filesystem path `bring_up` touches, gathered so tests can point the
/// whole chain at a tempdir instead of the real device tree. `production()`
/// reproduces the hard-coded paths above exactly; nothing here changes
/// on-device behaviour.
#[derive(Debug, Clone)]
pub struct FsLayout {
    pub hw_conf: String,
    pub hw_conf_factory: String,
    pub gpio_wifi_en: String,
    pub otg_module: String,
    pub wpa_conf: String,
    pub resolv_conf: String,
    pub ko_dir: String,
    pub sys_class_net: String,
    pub proc_fib_trie: String,
    pub proc_route: String,
    pub wifi_manage_script: String,
    pub wifi_driver_tgz: String,
    pub wifi_tool_tgz: String,
    pub busybox: String,
}

impl FsLayout {
    pub fn production() -> Self {
        Self {
            hw_conf: HW_CONF.into(),
            hw_conf_factory: HW_CONF_FACTORY.into(),
            gpio_wifi_en: GPIO_WIFI_EN.into(),
            otg_module: OTG_MODULE.into(),
            wpa_conf: WPA_CONF.into(),
            resolv_conf: RESOLV_CONF.into(),
            ko_dir: KO_DIR.into(),
            sys_class_net: SYS_CLASS_NET.into(),
            proc_fib_trie: PROC_FIB_TRIE.into(),
            proc_route: PROC_ROUTE.into(),
            wifi_manage_script: WIFI_MANAGE_SCRIPT.into(),
            wifi_driver_tgz: WIFI_DRIVER_TGZ.into(),
            wifi_tool_tgz: WIFI_TOOL_TGZ.into(),
            busybox: BUSYBOX.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Up {
        chip: &'static str,
        ssid: String,
        addr: String,
        /// The `-D` flag that actually associated. The supervised service in P3
        /// must be started with this one, not with the config's default (R8).
        driver: &'static str,
    },
    /// Bring-up failed and the vendor chain was invoked instead (R7).
    FellBack,
    /// Bring-up failed and the fallback was disabled by config.
    Failed,
}

/// Resolve the chip and GPIO polarity. Pinned config wins; `auto` parses
/// `hw.conf`, preferring the Factory override when present
/// (`wifi_driver.sh:41-47`).
pub fn resolve_chip(pinned: &str, hw_conf_path: &str) -> Result<(&'static Chip, Polarity), String> {
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

/// Clear the Wi-Fi reboot counter after a successful association (B4).
fn on_association_success(storm_state_path: &str, outcome: Outcome) -> Outcome {
    if matches!(outcome, Outcome::Up { .. }) {
        let mut storm = crate::storm::StormState::load(storm_state_path);
        storm.wifi_reboots = 0;
        if let Err(e) = storm.save(storm_state_path) {
            tracing::warn!(error = %e, "failed to clear wifi reboot counter");
        }
    }
    outcome
}

/// Full bring-up. Steps are numbered to match the design addendum.
///
/// `storm_state_path` is where the wifi reboot counter lives; a successful
/// bring-up is the only thing that zeroes it (B4).
pub fn bring_up(sys: &dyn Sys, cfg: &WifiCfg, storm_state_path: &str) -> Outcome {
    bring_up_with(sys, cfg, storm_state_path, &FsLayout::production())
}

/// Same as [`bring_up`], but with the filesystem paths it touches taken from
/// `layout` instead of the hard-coded device tree. Exists so tests can run
/// the whole chain against a tempdir; production always calls [`bring_up`].
pub fn bring_up_with(
    sys: &dyn Sys,
    cfg: &WifiCfg,
    storm_state_path: &str,
    layout: &FsLayout,
) -> Outcome {
    match try_bring_up_with(sys, cfg, layout) {
        Ok(outcome) => on_association_success(storm_state_path, outcome),
        Err(e) => {
            tracing::error!(error = %e, "wifi bring-up failed");
            if cfg.fallback_to_vendor {
                fall_back(sys, layout)
            } else {
                tracing::error!("fallback_to_vendor is disabled; the camera may be unreachable");
                Outcome::Failed
            }
        }
    }
}

/// [`bring_up_with`], plus rung 2 of the rescue ladder.
///
/// `gateway_reachable` already rescues a bad static address. It cannot rescue
/// bad credentials: with no association there is no gateway to probe. So when
/// bring-up fails outright *and* an overlay is what produced `cfg`, quarantine
/// the overlay and retry once with the operator's baseline before falling
/// through to the vendor chain.
pub fn bring_up_with_overlay(
    sys: &dyn Sys,
    cfg: &WifiCfg,
    baseline: &WifiCfg,
    storm_state_path: &str,
    layout: &FsLayout,
    overlay_path: &std::path::Path,
) -> Outcome {
    match try_bring_up_with(sys, cfg, layout) {
        Ok(outcome) => on_association_success(storm_state_path, outcome),
        Err(e) => {
            tracing::error!(error = %e, "wifi bring-up failed");

            if overlay_path.exists() {
                if crate::netoverlay::NetworkOverlay::load(overlay_path)
                    .is_ok_and(|overlay| overlay.overrides_association())
                {
                    crate::netoverlay::NetworkOverlay::quarantine(overlay_path);
                }

                match try_bring_up_with(sys, baseline, layout) {
                    Ok(outcome) => {
                        tracing::warn!("recovered on the baseline config");
                        return on_association_success(storm_state_path, outcome);
                    }
                    Err(e) => tracing::error!(error = %e, "baseline retry also failed"),
                }
            }

            if cfg.fallback_to_vendor {
                fall_back(sys, layout)
            } else {
                tracing::error!("fallback_to_vendor is disabled; the camera may be unreachable");
                Outcome::Failed
            }
        }
    }
}

/// R7. Regenerates the vendor-shaped conf before delegating, because the
/// RTL8188 path `sed`s into line numbers 3 and 4 (R10).
fn fall_back(sys: &dyn Sys, layout: &FsLayout) -> Outcome {
    tracing::error!(
        script = %layout.wifi_manage_script,
        "falling back to the vendor wifi chain"
    );
    match sys.run_to_completion(&layout.wifi_manage_script, &["start".to_string()]) {
        Ok(st) => tracing::warn!(?st, "vendor wifi_manage.sh start invoked"),
        Err(e) => tracing::error!(error = %e, "vendor fallback also failed"),
    }
    Outcome::FellBack
}

fn try_bring_up_with(sys: &dyn Sys, cfg: &WifiCfg, layout: &FsLayout) -> Result<Outcome, String> {
    // 1. Resolve chip.
    let hw_path = if std::path::Path::new(&layout.hw_conf_factory).exists() {
        &layout.hw_conf_factory
    } else {
        &layout.hw_conf
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
        std::fs::write(&layout.gpio_wifi_en, level)
            .map_err(|e| format!("gpio write {}={level}: {e}", layout.gpio_wifi_en))?;
        sys.sleep(Duration::from_secs(1));
    }

    // 4. Prepare (wifi_driver.sh:104-112, 197).
    std::fs::create_dir_all(&layout.ko_dir).map_err(|e| format!("mkdir {}: {e}", layout.ko_dir))?;
    let _ = sys.run_to_completion(
        "tar",
        &[
            "zxf".into(),
            layout.wifi_driver_tgz.clone(),
            "-C".into(),
            layout.ko_dir.clone(),
        ],
    );
    let _ = sys.run_to_completion(
        "tar",
        &[
            "zxf".into(),
            layout.wifi_tool_tgz.clone(),
            "-C".into(),
            "/tmp".into(),
        ],
    );
    let _ = sys.insmod(&layout.otg_module);
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
    wait_for_interface(sys, &cfg.interface, Duration::from_secs(30), layout)?;
    sys.run_to_completion("ifconfig", &[cfg.interface.clone(), "up".into()])
        .map_err(|e| format!("ifconfig up: {e}"))?;

    // 7. Write wpa_supplicant.conf.
    std::fs::write(
        &layout.wpa_conf,
        wpa_supplicant_conf(&cfg.ssid, &cfg.password, sec),
    )
    .map_err(|e| format!("write {}: {e}", layout.wpa_conf))?;

    // 8-9. wpa_supplicant is a supervised service started in P3 (design Q1), so
    // bring-up starts it once here in the foreground-detached form and waits for
    // association. See Task A8 for the driver-flag probe (R8).
    let driver = start_supplicant_probing_driver(sys, cfg, layout)?;
    tracing::info!(driver, "wpa_supplicant associated");

    // 10-12: address, resolv.conf, verification.
    let addr = assign_address(sys, cfg, layout)?;
    if !cfg.dns.is_empty() {
        std::fs::write(&layout.resolv_conf, resolv_conf(&cfg.dns))
            .map_err(|e| format!("write {}: {e}", layout.resolv_conf))?;
    }

    Ok(Outcome::Up {
        chip: chip.name,
        ssid: cfg.ssid.clone(),
        addr,
        driver,
    })
}

fn wait_for_interface(
    sys: &dyn Sys,
    iface: &str,
    cap: Duration,
    layout: &FsLayout,
) -> Result<(), String> {
    let path = format!("{}/{iface}", layout.sys_class_net);
    let deadline = sys.now() + cap;
    while sys.now() < deadline {
        if std::path::Path::new(&path).exists() {
            return Ok(());
        }
        sys.sleep(Duration::from_secs(1));
    }
    Err(format!("{iface} did not appear within {cap:?}"))
}

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
/// lost its symlinks (R17). Spawned as `/bin/busybox` with `udhcpc` as the
/// first argument so busybox resolves the applet from argv[1].
pub fn udhcpc_oneshot_args(iface: &str) -> Vec<String> {
    vec![
        "udhcpc".into(),
        "-i".into(),
        iface.into(),
        "-n".into(),
        "-q".into(),
    ]
}

/// Rewrite the `-D <driver>` pair in a service argv. Returns false when the
/// service does not take a `-D` flag, which the caller logs.
pub fn patch_driver_arg(args: &mut [String], driver: &str) -> bool {
    let Some(i) = args.iter().position(|a| a == "-D") else {
        return false;
    };
    let Some(slot) = args.get_mut(i + 1) else {
        return false;
    };
    slot.clear();
    slot.push_str(driver);
    true
}

fn start_supplicant_probing_driver(
    sys: &dyn Sys,
    cfg: &WifiCfg,
    layout: &FsLayout,
) -> Result<&'static str, String> {
    let timeout = Duration::from_secs(cfg.connect_timeout_sec);
    for driver in DRIVER_PROBE_ORDER {
        tracing::info!(driver, "starting wpa_supplicant");
        let _ = sys.run_to_completion("killall", &["wpa_supplicant".into()]);
        // Spawned detached here only for the duration of bring-up. P3 registers
        // it as a supervised service with whichever driver flag worked.
        sys.spawn_detached(SUPPLICANT_BIN, &supplicant_args(&cfg.interface, driver))
            .map_err(|e| format!("spawn wpa_supplicant: {e}"))?;
        if wait_associated(sys, &cfg.interface, timeout, layout) {
            return Ok(driver);
        }
        tracing::warn!(driver, "no association; trying the next driver flag");
    }
    Err("no driver flag produced an association".into())
}

fn wait_associated(sys: &dyn Sys, iface: &str, cap: Duration, layout: &FsLayout) -> bool {
    let deadline = sys.now() + cap;
    while sys.now() < deadline {
        if read_carrier(iface, layout) == Some(true) {
            return true;
        }
        sys.sleep(Duration::from_secs(1));
    }
    false
}

fn assign_address(sys: &dyn Sys, cfg: &WifiCfg, layout: &FsLayout) -> Result<String, String> {
    if cfg.dhcp {
        return dhcp_once(sys, cfg, layout);
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
    if gateway_reachable(&gw) {
        return Ok(cidr.address);
    }
    tracing::error!(
        gateway = %gw,
        "static address assigned but gateway unreachable; retrying via DHCP"
    );
    dhcp_once(sys, cfg, layout)
}

fn dhcp_once(sys: &dyn Sys, cfg: &WifiCfg, layout: &FsLayout) -> Result<String, String> {
    sys.run_to_completion(&layout.busybox, &udhcpc_oneshot_args(&cfg.interface))
        .map_err(|e| format!("udhcpc: {e}"))?;
    let fib = std::fs::read_to_string(&layout.proc_fib_trie).unwrap_or_default();
    let route = std::fs::read_to_string(&layout.proc_route).unwrap_or_default();
    crate::netstat::parse_local_ipv4(&fib, &route, &cfg.interface)
        .ok_or_else(|| format!("no address on {} after udhcpc", cfg.interface))
}

fn read_carrier(iface: &str, layout: &FsLayout) -> Option<bool> {
    let src = std::fs::read_to_string(format!("{}/{iface}/carrier", layout.sys_class_net)).ok()?;
    Some(src.trim() == "1")
}

fn gateway_reachable(gw: &str) -> bool {
    crate::netstat::gateway_reachable(gw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sys::{ExitStatus, MockSys};

    fn test_layout(dir: &std::path::Path) -> FsLayout {
        let p = |name: &str| dir.join(name).to_str().expect("utf8").to_string();
        FsLayout {
            hw_conf: p("hw.conf"),
            hw_conf_factory: p("hw.conf.factory"),
            gpio_wifi_en: p("wifi_en"),
            otg_module: p("otg.ko"),
            wpa_conf: p("wpa_supplicant.conf"),
            resolv_conf: p("resolv.conf"),
            ko_dir: p("ko"),
            sys_class_net: p("net"),
            proc_fib_trie: p("fib_trie"),
            proc_route: p("route"),
            wifi_manage_script: p("wifi_manage.sh"),
            wifi_driver_tgz: p("wifi_driver.tgz"),
            wifi_tool_tgz: p("wifi_tool.tgz"),
            busybox: "busybox".to_string(),
        }
    }

    fn happy_cfg() -> WifiCfg {
        WifiCfg {
            ssid: "testnet".into(),
            password: "password1".into(),
            config_file: "/dev/null".into(),
            chip: "ssv6355_ble".into(),
            gpio_polarity: "low_high".into(),
            interface: "wlan0".into(),
            security: "wpa".into(),
            dhcp: true,
            address: None,
            gateway: None,
            dns: vec!["192.168.2.1".into()],
            connect_timeout_sec: 5,
            fallback_to_vendor: true,
        }
    }

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

    // The real record from this camera: orig/etc/jffs2/hw.conf, byte-identical to
    // the /mnt/Factory copy. 64 characters after the HW= prefix.
    const HW_REAL: &str = "HW=111513155011100180020000000000000000000000020000003h200000000000\n";

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
        assert!(
            out.contains("ctrl_interface="),
            "wpa_cli needs a control socket"
        );
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

    /// Without `scan_ssid=1` wpa_supplicant only associates to a network it
    /// has heard a beacon from. With it, it sends a directed probe request and
    /// associates on the probe response. On .121 the camera sits between a
    /// router at -49 dBm and a mesh satellite at -85 dBm and roams between
    /// them; at the far end, waiting for a beacon inside the 45 s
    /// connect_timeout is unreliable. Also required outright if an SSID is ever
    /// configured not to broadcast.
    #[test]
    fn test_wpa_supplicant_conf_probes_actively_for_the_ssid() {
        for sec in [Security::Wpa, Security::Wep, Security::Open] {
            let out = wpa_supplicant_conf("net", "password", sec);
            assert!(
                out.contains("scan_ssid=1"),
                "every security mode must probe actively, missing for {sec:?}"
            );
        }
    }

    #[test]
    fn test_wpa_supplicant_conf_is_deterministic() {
        let a = wpa_supplicant_conf("net", "password", Security::Wpa);
        let b = wpa_supplicant_conf("net", "password", Security::Wpa);
        assert_eq!(a, b);
    }

    #[test]
    fn test_parse_cidr_splits_address_and_netmask() {
        let a = parse_cidr("192.168.2.198/24").expect("valid cidr");
        assert_eq!(a.address, "192.168.2.198");
        assert_eq!(a.netmask, "255.255.255.0");
    }

    #[test]
    fn test_parse_cidr_handles_uncommon_prefixes() {
        assert_eq!(parse_cidr("10.0.0.1/8").expect("/8").netmask, "255.0.0.0");
        assert_eq!(
            parse_cidr("10.0.0.1/30").expect("/30").netmask,
            "255.255.255.252"
        );
        assert_eq!(
            parse_cidr("10.0.0.1/32").expect("/32").netmask,
            "255.255.255.255"
        );
    }

    #[test]
    fn test_parse_cidr_rejects_malformed_input() {
        // R12: every one of these, accepted, produces an unreachable camera.
        for bad in [
            "192.168.2.198",    // no prefix
            "192.168.2.198/33", // prefix out of range
            "192.168.2/24",     // too few octets
            "192.168.2.999/24", // octet out of range
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

    #[test]
    fn test_supplicant_args_uses_probed_driver_and_no_dash_b() {
        // Q1: -B self-daemonizes, which the supervisor would read as an instant
        // crash and backoff-loop on forever. wifi_station.sh:60 already proves
        // foreground operation works on this hardware.
        let args = supplicant_args("wlan0", "nl80211");
        assert!(!args.contains(&"-B".to_string()), "must not self-daemonize");
        assert_eq!(
            args,
            vec![
                "-i".to_string(),
                "wlan0".to_string(),
                "-D".to_string(),
                "nl80211".to_string(),
                "-c".to_string(),
                WPA_CONF.to_string(),
            ]
        );
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
            vec![
                "wlan0".to_string(),
                "192.168.2.198".to_string(),
                "netmask".to_string(),
                "255.255.255.0".to_string(),
            ]
        );
    }

    #[test]
    fn test_udhcpc_oneshot_args_exit_after_lease() {
        // -n exits if no lease, -q quits once one is obtained. Without both, the
        // one-shot never returns and P2 blocks forever.
        let args = udhcpc_oneshot_args("wlan0");
        assert!(args.contains(&"-n".to_string()));
        assert!(args.contains(&"-q".to_string()));
        assert_eq!(
            args[0], "udhcpc",
            "busybox multicall: first arg selects the applet when exec is /bin/busybox"
        );
    }

    #[test]
    fn test_patch_driver_arg_replaces_the_flag_value() {
        let mut args = vec![
            "-i".to_string(),
            "wlan0".to_string(),
            "-D".to_string(),
            "nl80211".to_string(),
            "-c".to_string(),
            "x".to_string(),
        ];
        assert!(patch_driver_arg(&mut args, "wext"));
        assert_eq!(args[3], "wext");
    }

    #[test]
    fn test_patch_driver_arg_leaves_other_args_untouched() {
        let mut args = vec![
            "-i".to_string(),
            "wlan0".to_string(),
            "-D".to_string(),
            "nl80211".to_string(),
            "-c".to_string(),
            "x".to_string(),
        ];
        let before = args.clone();
        assert!(patch_driver_arg(&mut args, "wext"));
        assert_eq!(args.len(), before.len());
        for (i, (a, b)) in args.iter().zip(before.iter()).enumerate() {
            if i == 3 {
                continue;
            }
            assert_eq!(a, b);
        }
    }

    #[test]
    fn test_patch_driver_arg_returns_false_without_a_d_flag() {
        let mut args = vec!["-i".to_string(), "wlan0".to_string()];
        let before = args.clone();
        assert!(!patch_driver_arg(&mut args, "wext"));
        assert_eq!(args, before);
    }

    #[test]
    fn test_patch_driver_arg_ignores_a_trailing_d_flag() {
        let mut args = vec!["-D".to_string()];
        let before = args.clone();
        assert!(!patch_driver_arg(&mut args, "wext"));
        assert_eq!(args, before);
    }

    #[test]
    fn test_bring_up_returns_failed_when_chip_unknown_and_fallback_disabled() {
        let mut cfg = happy_cfg();
        cfg.chip = "nonexistent".into();
        cfg.fallback_to_vendor = false;
        // No sys expectations: an unknown pinned chip fails before any syscall.
        let sys = MockSys::new();
        assert_eq!(
            bring_up(&sys, &cfg, "/nonexistent/storm.json"),
            Outcome::Failed
        );
    }

    #[test]
    fn test_bring_up_returns_fellback_when_chip_unknown_and_fallback_enabled() {
        let mut cfg = happy_cfg();
        cfg.chip = "nonexistent".into();
        cfg.fallback_to_vendor = true;

        let mut sys = MockSys::new();
        sys.expect_run_to_completion()
            .withf(|prog, args| prog == "/usr/sbin/wifi_manage.sh" && args == ["start".to_string()])
            .times(1)
            .returning(|_, _| Ok(ExitStatus::Code(0)));

        assert_eq!(
            bring_up(&sys, &cfg, "/nonexistent/storm.json"),
            Outcome::FellBack
        );
    }

    /// Local fixtures mirroring `netstat.rs`'s: a wlan0 default route plus one
    /// `/32 host LOCAL` address, so `dhcp_once` -> `parse_local_ipv4` resolves.
    const HAPPY_ROUTE: &str = "\
Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT
wlan0\t00000000\t0102A8C0\t0003\t0\t0\t0\t00000000\t0\t0\t0
wlan0\t0002A8C0\t00000000\t0001\t0\t0\t0\t00FFFFFF\t0\t0\t0
";
    const HAPPY_FIB_TRIE: &str = "\
Local:
  +-- 0.0.0.0/0 3 0 4
     |-- 0.0.0.0
        /0 universe UNICAST
     +-- 192.168.2.0/24 2 0 2
        |-- 192.168.2.198
           /32 host LOCAL
";

    fn happy_mock_sys() -> MockSys {
        let mut sys = MockSys::new();
        sys.expect_now().returning(std::time::Instant::now);
        sys.expect_sleep().returning(|_| ());
        sys.expect_insmod().returning(|_| Ok(()));
        sys.expect_rmmod().returning(|_| Ok(()));
        sys.expect_run_to_completion()
            .returning(|_, _| Ok(ExitStatus::Code(0)));
        sys.expect_spawn_detached().returning(|_, _| Ok(4242));
        sys
    }

    #[test]
    fn test_try_bring_up_with_happy_path_over_dhcp() {
        let dir = tempfile::tempdir().expect("tempdir");
        let layout = test_layout(dir.path());
        std::fs::create_dir_all(format!("{}/wlan0", layout.sys_class_net)).expect("iface dir");
        std::fs::write(format!("{}/wlan0/carrier", layout.sys_class_net), "1").expect("carrier");
        std::fs::write(&layout.proc_route, HAPPY_ROUTE).expect("route");
        std::fs::write(&layout.proc_fib_trie, HAPPY_FIB_TRIE).expect("fib_trie");

        let cfg = happy_cfg();
        let sys = happy_mock_sys();

        let outcome = try_bring_up_with(&sys, &cfg, &layout).expect("bring-up must succeed");
        match outcome {
            Outcome::Up {
                chip,
                ssid,
                addr,
                driver,
            } => {
                assert_eq!(chip, "ssv6355_ble");
                assert_eq!(ssid, "testnet");
                assert_eq!(addr, "192.168.2.198");
                assert_eq!(driver, "nl80211", "first driver in the probe order");
            }
            other => panic!("expected Outcome::Up, got {other:?}"),
        }

        let wpa = std::fs::read_to_string(&layout.wpa_conf).expect("wpa conf written");
        assert!(wpa.contains(r#"ssid="testnet""#));
        let resolv = std::fs::read_to_string(&layout.resolv_conf).expect("resolv conf written");
        assert_eq!(resolv, "nameserver 192.168.2.1\n");
    }

    #[test]
    fn test_bring_up_with_clears_wifi_reboot_counter_on_successful_association() {
        let dir = tempfile::tempdir().expect("tempdir");
        let layout = test_layout(dir.path());
        std::fs::create_dir_all(format!("{}/wlan0", layout.sys_class_net)).expect("iface dir");
        std::fs::write(format!("{}/wlan0/carrier", layout.sys_class_net), "1").expect("carrier");
        std::fs::write(&layout.proc_route, HAPPY_ROUTE).expect("route");
        std::fs::write(&layout.proc_fib_trie, HAPPY_FIB_TRIE).expect("fib_trie");

        let storm_path = dir.path().join("storm.json");
        std::fs::write(&storm_path, r#"{"fast_reboots":1,"wifi_reboots":2}"#)
            .expect("seed storm state");

        let cfg = happy_cfg();
        let sys = happy_mock_sys();

        let outcome = bring_up_with(&sys, &cfg, storm_path.to_str().expect("utf8"), &layout);
        assert!(
            matches!(outcome, Outcome::Up { .. }),
            "expected Outcome::Up, got {outcome:?}"
        );

        let storm = crate::storm::StormState::load(storm_path.to_str().expect("utf8"));
        assert_eq!(
            storm.wifi_reboots, 0,
            "B4: cleared only by a successful bring-up"
        );
        assert_eq!(
            storm.fast_reboots, 1,
            "bring-up must not touch the unrelated crash-loop counter"
        );
    }

    #[test]
    fn test_a_failing_overlay_is_quarantined_and_the_baseline_is_retried() {
        // Arrange: same tempdir layout as the happy-path test, but no carrier,
        // so association fails and bring-up returns Err.
        let dir = tempfile::tempdir().expect("tempdir");
        let layout = test_layout(dir.path());
        std::fs::create_dir_all(format!("{}/wlan0", layout.sys_class_net))
            .expect("iface dir");
        std::fs::write(&layout.proc_route, HAPPY_ROUTE).expect("route");
        std::fs::write(&layout.proc_fib_trie, HAPPY_FIB_TRIE).expect("fib_trie");

        let mut overlay_cfg = happy_cfg();
        overlay_cfg.ssid = "TypoNet".into();
        overlay_cfg.password = "password1".into();
        overlay_cfg.connect_timeout_sec = 0;
        overlay_cfg.fallback_to_vendor = false;

        let mut baseline_cfg = happy_cfg();
        baseline_cfg.ssid = "OperatorNet".into();
        baseline_cfg.password = "password1".into();
        baseline_cfg.connect_timeout_sec = 0;

        let storm_path = dir.path().join("storm.json");

        let overlay_path = dir.path().join("network.toml");
        std::fs::write(&overlay_path, "ssid = \"TypoNet\"\n").expect("write");

        let sys = happy_mock_sys();

        // Act
        let outcome = bring_up_with_overlay(
            &sys,
            &overlay_cfg,
            &baseline_cfg,
            storm_path.to_str().expect("utf8"),
            &layout,
            &overlay_path,
        );

        // Assert: quarantine + a visible baseline retry
        assert_eq!(outcome, Outcome::Failed);
        assert!(!overlay_path.exists(), "the failing overlay must be quarantined");

        let bad = dir.path().join("network.toml.bad");
        assert!(bad.exists(), "the quarantined overlay must be readable by the UI");
        assert_eq!(
            std::fs::read_to_string(&bad).expect("read"),
            "ssid = \"TypoNet\"\n",
            "quarantine must preserve the content so the UI can show what failed"
        );

        let wpa = std::fs::read_to_string(&layout.wpa_conf).expect("wpa conf written");
        assert!(
            wpa.contains("ssid=\"OperatorNet\""),
            "baseline retry must rewrite wpa_supplicant.conf"
        );
    }
}
