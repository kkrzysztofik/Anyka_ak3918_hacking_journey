//! Wifi bring-up, replacing the vendor chain: wifi_manage.sh -> wifi_run.sh ->
//! wifi_driver.sh / wifi_station.sh -> station_connect.sh (1,316 lines of sh).
//!
//! See docs/plans/2026-08-01-boot-runtime-rust-design.md, addendum.

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
}
