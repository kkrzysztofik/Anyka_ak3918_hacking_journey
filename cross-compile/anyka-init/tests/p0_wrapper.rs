//! The P0 wrapper is the one file whose failure cannot be fixed remotely:
//! .121 has no serial console and recovery means pulling the SD card on-site.
//! These tests run the shipped script under `sh` with stubbed helpers so every
//! deadman branch is exercised exactly as written.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../SD_card_contents/Factory/config.sh")
}

/// Write an executable stub into `dir`.
fn stub(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("write stub");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("chmod stub");
    path
}

/// What the deadman did, after the script has run to completion.
struct Outcome {
    vendor_wifi_started: bool,
    rebooted: bool,
    /// True when the vendor boot path was copied back over config.sh.
    boot_path_restored: bool,
    exit_ok: bool,
}

/// `ifconfig_addrs` is consulted once per call the script makes: the Nth entry
/// is what the Nth `ifconfig wlan0` reports. `None` means "no address".
/// This is what lets one test cover "vendor chain rescued it" and another
/// cover "still dead, roll back".
fn run(ifconfig_addrs: &[Option<&str>], binary_present: bool) -> Outcome {
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path();

    // `sleep 180` then `sleep 60` would make the test take four minutes.
    stub(dir, "sleep", "exit 0");
    stub(dir, "telnetd", "exit 0");
    stub(dir, "sync", "exit 0");

    // A counter file turns successive `ifconfig` calls into a scripted sequence.
    let counter = dir.join("ifconfig-calls");
    fs::write(&counter, "0").expect("seed counter");
    let cases: Vec<String> = ifconfig_addrs
        .iter()
        .enumerate()
        .map(|(i, addr)| match addr {
            Some(a) => format!("{i}) echo 'inet addr:{a}' ;;"),
            None => format!("{i}) echo '' ;;"),
        })
        .collect();
    stub(
        dir,
        "ifconfig",
        &format!(
            "n=$(cat '{c}'); echo $((n + 1)) > '{c}'\ncase $n in\n{cases}\n*) echo '' ;;\nesac",
            c = counter.display(),
            cases = cases.join("\n"),
        ),
    );

    let wifi_marker = dir.join("vendor-wifi-started");
    let wifi_manage = stub(
        dir,
        "wifi_manage.sh",
        &format!("touch '{}'", wifi_marker.display()),
    );

    let reboot_marker = dir.join("rebooted");
    stub(dir, "reboot", &format!("touch '{}'", reboot_marker.display()));

    // The vendor boot path the deadman restores, and the file it overwrites.
    let bak = dir.join("config.sh.gerge.bak");
    fs::write(&bak, "VENDOR BOOT PATH\n").expect("write bak");
    let self_path = dir.join("config.sh.live");
    fs::write(&self_path, "SUPERVISOR BOOT PATH\n").expect("write live");

    let init_bin = if binary_present {
        // The supervisor never returns in production; exiting keeps the test bounded.
        stub(dir, "anyka-init.bin", "exit 0")
    } else {
        dir.join("does-not-exist")
    };

    let status = Command::new("sh")
        .arg(script())
        .env("PATH", format!("{}:/usr/bin:/bin", dir.display()))
        .env("ANYKA_INIT_BIN", &init_bin)
        .env("ANYKA_WIFI_MANAGE", &wifi_manage)
        .env("ANYKA_CONFIG_SELF", &self_path)
        .env("ANYKA_CONFIG_BAK", &bak)
        .status()
        .expect("run config.sh");

    // The deadman is a backgrounded subshell; give it a moment to finish.
    std::thread::sleep(std::time::Duration::from_millis(400));

    Outcome {
        vendor_wifi_started: wifi_marker.exists(),
        rebooted: reboot_marker.exists(),
        boot_path_restored: fs::read_to_string(&self_path)
            .expect("read live")
            .contains("VENDOR BOOT PATH"),
        exit_ok: status.success(),
    }
}

#[test]
fn healthy_boot_leaves_everything_alone() {
    let o = run(&[Some("192.168.30.121")], true);
    assert!(o.exit_ok, "wrapper must exit 0 when the binary is present");
    assert!(!o.vendor_wifi_started, "an address is held; do not touch wifi");
    assert!(!o.rebooted, "a healthy camera must never be rebooted");
    assert!(!o.boot_path_restored, "the boot path must not be rolled back");
}

#[test]
fn no_address_hands_wifi_back_to_the_vendor_chain() {
    // First check: no address. Second check, after wifi_manage.sh: recovered.
    let o = run(&[None, Some("192.168.30.121")], true);
    assert!(o.vendor_wifi_started, "no IP must invoke the vendor chain");
    assert!(
        !o.rebooted,
        "the vendor chain rescued the link; rolling back would be premature"
    );
    assert!(!o.boot_path_restored);
}

/// The 2026-08-03 dry run: a wedged radio makes `wifi_manage.sh start` fail the
/// same way anyka-init's own fallback did. Only a reboot clears it, and it must
/// come back on a boot path that works.
#[test]
fn still_dead_after_the_vendor_chain_restores_the_boot_path_and_reboots() {
    let o = run(&[None, None], true);
    assert!(o.vendor_wifi_started, "the vendor chain must be tried first");
    assert!(
        o.boot_path_restored,
        "a still-dead link must restore the vendor boot path"
    );
    assert!(o.rebooted, "and reboot into it");
}

/// A missing binary exits non-zero, and service.sh's FACTORY_TEST branch then
/// returns without ever starting wifi. The deadman must already be armed by
/// then, or this is exactly the stranding it exists to prevent.
#[test]
fn missing_binary_fails_loudly_but_still_arms_the_deadman() {
    let o = run(&[None, None], false);
    assert!(!o.exit_ok, "a missing supervisor must be a loud failure");
    assert!(o.vendor_wifi_started, "the deadman must run despite the exit");
    assert!(o.boot_path_restored);
    assert!(o.rebooted);
}
