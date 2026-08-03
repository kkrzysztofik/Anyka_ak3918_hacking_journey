//! The P0 wrapper is the one file whose failure cannot be fixed remotely:
//! .121 has no serial console and recovery means pulling the SD card on-site.
//! These tests run the shipped script under `sh` with stubbed helpers so both
//! deadman branches are exercised exactly as written.

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

/// Run the wrapper with `ifconfig` reporting `has_ip`, and report whether the
/// deadman invoked the vendor wifi chain.
fn run(has_ip: bool) -> bool {
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path();

    // `sleep 180` would make the test take three minutes.
    stub(dir, "sleep", "exit 0");
    stub(dir, "telnetd", "exit 0");
    let ifconfig_out = if has_ip { "inet addr:192.168.30.121" } else { "" };
    stub(dir, "ifconfig", &format!("echo '{ifconfig_out}'"));

    let marker = dir.join("vendor-wifi-started");
    let wifi_manage = stub(
        dir,
        "wifi_manage.sh",
        &format!("touch '{}'", marker.display()),
    );
    // The supervisor never returns in production; exiting keeps the test bounded.
    let init_bin = stub(dir, "anyka-init.bin", "exit 0");

    let status = Command::new("sh")
        .arg(script())
        .env("PATH", format!("{}:/usr/bin:/bin", dir.display()))
        .env("ANYKA_INIT_BIN", &init_bin)
        .env("ANYKA_WIFI_MANAGE", &wifi_manage)
        .status()
        .expect("run config.sh");
    assert!(
        status.success(),
        "wrapper must exit 0 when the binary is present"
    );

    // The deadman is a backgrounded subshell; give it a moment to finish.
    std::thread::sleep(std::time::Duration::from_millis(300));
    marker.exists()
}

#[test]
fn deadman_stays_quiet_when_the_interface_has_an_address() {
    assert!(
        !run(true),
        "wlan0 has an IP, so the vendor chain must not be started"
    );
}

#[test]
fn deadman_restores_vendor_wifi_when_the_interface_has_no_address() {
    assert!(
        run(false),
        "no IP after the delay must hand wifi back to the vendor chain"
    );
}

/// A missing binary exits non-zero, and service.sh's FACTORY_TEST branch then
/// returns without ever starting wifi. The deadman must already be armed by
/// then, or this is exactly the stranding it exists to prevent.
#[test]
fn wrapper_fails_loudly_but_still_arms_the_deadman_when_the_binary_is_missing() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path();
    stub(dir, "sleep", "exit 0");
    stub(dir, "telnetd", "exit 0");
    stub(dir, "ifconfig", "echo ''");

    let marker = dir.join("vendor-wifi-started");
    let wifi_manage = stub(
        dir,
        "wifi_manage.sh",
        &format!("touch '{}'", marker.display()),
    );

    let status = Command::new("sh")
        .arg(script())
        .env("PATH", format!("{}:/usr/bin:/bin", dir.display()))
        .env("ANYKA_INIT_BIN", dir.join("does-not-exist"))
        .env("ANYKA_WIFI_MANAGE", &wifi_manage)
        .status()
        .expect("run config.sh");
    assert!(
        !status.success(),
        "a missing supervisor must be a loud failure"
    );

    std::thread::sleep(std::time::Duration::from_millis(300));
    assert!(
        marker.exists(),
        "the deadman must be armed before the -x guard can exit"
    );
}
