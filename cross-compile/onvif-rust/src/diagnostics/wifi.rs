//! Wi-Fi association snapshot for `/api/diagnostics`.

use std::collections::HashMap;
use std::process::Command;

use serde::Serialize;

pub const DEFAULT_WIFI_IFACE: &str = "wlan0";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WifiDiagnostics {
    pub interface: String,
    pub connected: bool,
    pub ssid: Option<String>,
    pub frequency_mhz: Option<u32>,
    pub channel: Option<u32>,
    pub security: Option<String>,
    pub signal_dbm: Option<i32>,
}

/// Map `wpa_cli status` key_mgmt values to short UI labels.
pub fn map_key_mgmt(raw: &str) -> String {
    match raw {
        "WPA2-PSK" => "WPA2".to_string(),
        "WPA-PSK" => "WPA".to_string(),
        "NONE" => "Open".to_string(),
        "WPA2-EAP" | "WPA-EAP" => "Enterprise".to_string(),
        other => other.to_string(),
    }
}

/// Convert centre frequency (MHz) to IEEE channel when unambiguous.
pub fn frequency_to_channel(freq_mhz: u32) -> Option<u32> {
    if (2412..=2484).contains(&freq_mhz) && freq_mhz % 5 == 2 {
        return Some((freq_mhz - 2407) / 5);
    }
    if (5170..=5825).contains(&freq_mhz) && freq_mhz.is_multiple_of(5) {
        return Some((freq_mhz - 5000) / 5);
    }
    None
}

/// Parse `wpa_cli -i <iface> status` key=value output.
pub fn parse_wpa_status(text: &str, interface: &str) -> WifiDiagnostics {
    let mut fields = HashMap::new();
    for line in text.lines() {
        if let Some((key, value)) = line.split_once('=') {
            fields.insert(key.trim(), value.trim());
        }
    }

    let wpa_state = fields.get("wpa_state").copied().unwrap_or("");
    let connected = wpa_state == "COMPLETED";
    let frequency_mhz = fields.get("freq").and_then(|value| value.parse().ok());
    let channel = frequency_mhz.and_then(frequency_to_channel);
    let security = fields.get("key_mgmt").map(|value| map_key_mgmt(value));
    let ssid = fields.get("ssid").map(|value| value.to_string());
    let signal_dbm = fields.get("signal").and_then(|value| value.parse().ok());

    WifiDiagnostics {
        interface: interface.to_string(),
        connected,
        ssid,
        frequency_mhz,
        channel,
        security,
        signal_dbm,
    }
}

/// Read live Wi-Fi status via `wpa_cli`. Returns `None` when the tool or iface is absent.
pub fn read_wifi_diagnostics(interface: &str) -> Option<WifiDiagnostics> {
    let output = Command::new("wpa_cli")
        .args(["-i", interface, "status"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Some(parse_wpa_status(&text, interface))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
bssid=3c:64:cf:7d:a1:9f
freq=2437
ssid=kmk
id=0
mode=station
wifi_generation=4
pairwise_cipher=CCMP
group_cipher=CCMP
key_mgmt=WPA2-PSK
wpa_state=COMPLETED
ip_address=192.168.2.198
signal=-52
";

    #[test]
    fn parse_wpa_status_maps_connected_wifi() {
        let wifi = parse_wpa_status(SAMPLE, "wlan0");
        assert!(wifi.connected);
        assert_eq!(wifi.ssid.as_deref(), Some("kmk"));
        assert_eq!(wifi.frequency_mhz, Some(2437));
        assert_eq!(wifi.channel, Some(6));
        assert_eq!(wifi.security.as_deref(), Some("WPA2"));
        assert_eq!(wifi.signal_dbm, Some(-52));
    }

    #[test]
    fn frequency_to_channel_handles_common_2g4_values() {
        assert_eq!(frequency_to_channel(2412), Some(1));
        assert_eq!(frequency_to_channel(2437), Some(6));
    }

    #[test]
    fn map_key_mgmt_covers_open_and_enterprise() {
        assert_eq!(map_key_mgmt("NONE"), "Open");
        assert_eq!(map_key_mgmt("WPA2-EAP"), "Enterprise");
    }
}
