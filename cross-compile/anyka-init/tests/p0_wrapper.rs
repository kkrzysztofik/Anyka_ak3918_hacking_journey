//! The P0 wrapper is the one file whose failure cannot be fixed remotely:
//! .121 has no serial console and recovery means pulling the SD card on-site.
//! These tests run the shipped script under `sh` with stubbed helpers so every
//! deadman branch is exercised exactly as written.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
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
///
/// `expected_ifconfig_calls` is the number of `ifconfig` invocations the
/// deadman must make for the outcome to be final; `run` waits for that count
/// (with a bounded deadline) instead of a fixed sleep, so a loaded host cannot
/// observe the background subshell mid-flight.
///
/// `cp_fails` stubs `cp` to exit 1, exercising the restore-failure path.
fn run(
    ifconfig_addrs: &[Option<&str>],
    binary_present: bool,
    expected_ifconfig_calls: usize,
    cp_fails: bool,
) -> Outcome {
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path();

    // `sleep 180` then `sleep 60` would make the test take four minutes.
    stub(dir, "sleep", "exit 0");
    stub(dir, "telnetd", "exit 0");
    stub(dir, "sync", "exit 0");
    if cp_fails {
        stub(dir, "cp", "exit 1");
    }

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
    stub(
        dir,
        "reboot",
        &format!("touch '{}'", reboot_marker.display()),
    );

    // The vendor boot path the deadman restores, and the file it overwrites.
    let bak = dir.join("config.sh.gerge.bak");
    fs::write(&bak, "VENDOR BOOT PATH\n").expect("write bak");
    let self_path = dir.join("config.sh.live");
    fs::write(&self_path, "SUPERVISOR BOOT PATH\n").expect("write live");

    let init_bin = if binary_present {
        // The supervisor never returns in production; exiting keeps the test
        // bounded. config.sh now respawns it in a loop, so the whole wrapper
        // runs in its own process group and is killed once the assertions
        // have been observed (see below).
        stub(dir, "anyka-init.bin", "exit 0")
    } else {
        dir.join("does-not-exist")
    };

    let mut cmd = Command::new("sh");
    cmd.arg(script())
        .env("PATH", format!("{}:/usr/bin:/bin", dir.display()))
        .env("ANYKA_INIT_BIN", &init_bin)
        .env("ANYKA_WIFI_MANAGE", &wifi_manage)
        .env("ANYKA_CONFIG_SELF", &self_path)
        .env("ANYKA_CONFIG_BAK", &bak)
        // Own process group so the backgrounded respawn loop can be killed
        // with one signal once the test is done.
        .process_group(0);
    let mut child = cmd.spawn().expect("run config.sh");
    let child_pid = child.id() as i32;

    // The deadman is a backgrounded subshell. Wait for the expected number of
    // `ifconfig` calls (proving the watchdog actually ran, not just that a
    // sleep elapsed), then wait until the outcome stops changing — the
    // restore + reboot run right after the last call, and a loaded host must
    // not observe the subshell mid-restore.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut last = None;
    let observed = loop {
        let state = (
            fs::read_to_string(&counter)
                .ok()
                .and_then(|s| s.trim().parse::<usize>().ok())
                .unwrap_or(0),
            fs::read_to_string(&self_path)
                .ok()
                .map(|s| s.contains("VENDOR BOOT PATH"))
                .unwrap_or(false),
            reboot_marker.exists(),
        );
        if state.0 >= expected_ifconfig_calls && Some(state) == last {
            break state.0;
        }
        last = Some(state);
        if std::time::Instant::now() >= deadline {
            break state.0;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    assert!(
        observed >= expected_ifconfig_calls,
        "deadman made {observed} ifconfig calls, expected {expected_ifconfig_calls}, within 5s"
    );

    // The wrapper sh exits on its own once the loop is backgrounded; wait for
    // that natural exit so `exit_ok` reflects the script's real status.
    let status = child.wait().expect("wait config.sh");

    // Reap the backgrounded respawn loop, which spins forever against the
    // `exit 0` stub (binary + sleep both exit immediately) and would otherwise
    // burn a host CPU core. Killing the process group reaches it even though
    // the group leader already exited.
    // SAFETY: child_pid is our own spawned sh; a negative pid targets its
    // process group. No pointers are dereferenced.
    unsafe {
        libc::kill(-child_pid, libc::SIGKILL);
    }

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
    let o = run(&[Some("192.168.30.121")], true, 1, false);
    assert!(o.exit_ok, "wrapper must exit 0 when the binary is present");
    assert!(
        !o.vendor_wifi_started,
        "an address is held; do not touch wifi"
    );
    assert!(!o.rebooted, "a healthy camera must never be rebooted");
    assert!(
        !o.boot_path_restored,
        "the boot path must not be rolled back"
    );
}

#[test]
fn no_address_hands_wifi_back_to_the_vendor_chain() {
    // First check: no address. Second check, after wifi_manage.sh: recovered.
    let o = run(&[None, Some("192.168.30.121")], true, 2, false);
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
    let o = run(&[None, None], true, 2, false);
    assert!(
        o.vendor_wifi_started,
        "the vendor chain must be tried first"
    );
    assert!(
        o.boot_path_restored,
        "a still-dead link must restore the vendor boot path"
    );
    assert!(o.rebooted, "and reboot into it");
}

/// A failed restore must NOT reboot: rebooting after a failed `cp` would come
/// back on the unchanged broken wrapper.
#[test]
fn failed_boot_path_restore_does_not_reboot() {
    let o = run(&[None, None], true, 2, true);
    assert!(o.vendor_wifi_started);
    assert!(!o.rebooted, "a failed restore must never reboot");
    assert!(
        !o.boot_path_restored,
        "a failed copy must not have replaced the boot path"
    );
}

/// A missing binary exits non-zero, and service.sh's FACTORY_TEST branch then
/// returns without ever starting wifi. The deadman must already be armed by
/// then, or this is exactly the stranding it exists to prevent.
#[test]
fn missing_binary_fails_loudly_but_still_arms_the_deadman() {
    let o = run(&[None, None], false, 2, false);
    assert!(!o.exit_ok, "a missing supervisor must be a loud failure");
    assert!(
        o.vendor_wifi_started,
        "the deadman must run despite the exit"
    );
    assert!(o.boot_path_restored);
    assert!(o.rebooted);
}

/// Run config.sh against an A/B slot layout. Which slot runs is observed by
/// which marker stub the selected `anyka-init.bin` touches.
struct SlotOutcome {
    slot_a_ran: bool,
    slot_b_ran: bool,
}

/// `slot_b_present` controls whether `slots/b/anyka-init.bin` exists at all;
/// `pointer` is the content of `active` (`None` = no pointer file).
fn run_slot(slot_b_present: bool, pointer: Option<&str>) -> SlotOutcome {
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path();

    stub(dir, "sleep", "exit 0");
    stub(dir, "telnetd", "exit 0");
    stub(dir, "sync", "exit 0");
    stub(dir, "ifconfig", "echo 'inet addr:192.168.30.121'");

    let marker_a = dir.join("slot-a-ran");
    let marker_b = dir.join("slot-b-ran");
    fs::create_dir_all(dir.join("slots/a")).expect("slots/a");
    let _bin_a = stub(
        &dir.join("slots/a"),
        "anyka-init.bin",
        &format!("touch '{}'", marker_a.display()),
    );
    if slot_b_present {
        fs::create_dir_all(dir.join("slots/b")).expect("slots/b");
        let _ = stub(
            &dir.join("slots/b"),
            "anyka-init.bin",
            &format!("touch '{}'", marker_b.display()),
        );
    }
    if let Some(p) = pointer {
        fs::write(dir.join("active"), p).expect("write active pointer");
    }

    let mut cmd = Command::new("sh");
    cmd.arg(script())
        .env("PATH", format!("{}:/usr/bin:/bin", dir.display()))
        // Deliberately unset ANYKA_INIT_BIN: the default resolves from the slot
        // pointer, which is what these tests exercise.
        .env_remove("ANYKA_INIT_BIN")
        .env("ANYKA_SLOT_ROOT", dir)
        .env("ANYKA_WIFI_MANAGE", dir.join("wifi_manage.sh"))
        .env("ANYKA_CONFIG_SELF", dir.join("config.sh.live"))
        .env("ANYKA_CONFIG_BAK", dir.join("config.sh.gerge.bak"))
        .process_group(0);
    let mut child = cmd.spawn().expect("run config.sh");
    let child_pid = child.id() as i32;

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let outcome = loop {
        let o = SlotOutcome {
            slot_a_ran: marker_a.exists(),
            slot_b_ran: marker_b.exists(),
        };
        if o.slot_a_ran || o.slot_b_ran {
            break o;
        }
        if std::time::Instant::now() >= deadline {
            break SlotOutcome {
                slot_a_ran: false,
                slot_b_ran: false,
            };
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };

    let _ = child.wait().expect("wait config.sh");
    // SAFETY: child_pid is our own spawned sh; a negative pid targets its
    // process group (the backgrounded respawn loop).
    unsafe {
        libc::kill(-child_pid, libc::SIGKILL);
    }
    outcome
}

#[test]
fn boots_the_slot_named_by_the_pointer() {
    let o = run_slot(true, Some("b\n"));
    assert!(o.slot_b_ran, "active=b must run slots/b/anyka-init.bin");
    assert!(!o.slot_a_ran);
}

#[test]
fn falls_back_to_the_other_slot_when_the_named_one_is_missing() {
    // active points at b, but only slots/a exists.
    let o = run_slot(false, Some("b\n"));
    assert!(o.slot_a_ran, "missing named slot must fall back to slots/a");
    assert!(!o.slot_b_ran);
}

#[test]
fn defaults_to_slot_a_when_the_pointer_is_absent() {
    let o = run_slot(true, None);
    assert!(o.slot_a_ran, "no active file must default to slots/a");
    assert!(!o.slot_b_ran);
}
