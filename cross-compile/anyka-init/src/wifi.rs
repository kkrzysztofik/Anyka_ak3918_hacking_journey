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
