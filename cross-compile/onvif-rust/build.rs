//! Build script for ONVIF Rust implementation.
//!
//! Sets `use_stubs` cfg for host (non-ARM) builds so the crate compiles
//! with stub types and no-op implementations for testing.  ARM builds
//! use the real VendorIpc / PTZ driver paths.

use std::env;

fn main() {
    println!("cargo:rustc-check-cfg=cfg(use_stubs)");
    println!("cargo:rerun-if-changed=build.rs");

    let target = env::var("TARGET").unwrap_or_default();
    let is_arm = target.contains("arm") || target.contains("uclibc");

    if is_arm {
        // System libraries needed by the Rust runtime on ARM uClibc
        println!("cargo:rustc-link-lib=pthread");
        println!("cargo:rustc-link-lib=m");
        println!("cargo:rustc-link-lib=dl");
    } else {
        // Host builds use stub implementations for testing
        println!("cargo:rustc-cfg=use_stubs");
    }

    // The version the binary reports as FirmwareVersion. Honor an explicit
    // `ANYKA_BUILD_VERSION` (set by the bundle pipeline so manifest.meta and
    // the binary agree on one captured `git describe`), falling back to a
    // fresh `git describe` for ad-hoc builds.
    let v = match env::var("ANYKA_BUILD_VERSION") {
        Ok(v) if !v.trim().is_empty() => v.trim().to_string(),
        _ => std::process::Command::new("git")
            .args(["describe", "--tags", "--always", "--dirty"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|| "unknown".into()),
    };
    println!("cargo:rustc-env=ANYKA_BUILD_VERSION={v}");
    if env::var_os("ANYKA_BUILD_VERSION").is_some() {
        println!("cargo:rerun-if-env-changed=ANYKA_BUILD_VERSION");
    }

    // `.git/HEAD` alone is not enough to rerun on a commit advance: on a branch
    // it holds the constant line `ref: refs/heads/<branch>`, so its mtime only
    // changes on a branch switch. Watch the resolved ref file, packed-refs,
    // and the tag refs a `git describe` would walk, or `ANYKA_BUILD_VERSION`
    // goes stale across incremental builds.
    let git_dir = std::process::Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string());
    if let Some(dir) = &git_dir {
        println!("cargo:rerun-if-changed={dir}/HEAD");
        println!("cargo:rerun-if-changed={dir}/packed-refs");
        // `symbolic-ref` output is relative to the git dir.
        if let Ok(o) = std::process::Command::new("git")
            .args(["symbolic-ref", "-q", "HEAD"])
            .output()
            && o.status.success()
            && let Ok(ref_name) = String::from_utf8(o.stdout)
        {
            println!(
                "cargo:rerun-if-changed={dir}/{name}",
                name = ref_name.trim()
            );
        }
    }
    // Dereferenced tag objects (`git describe` uses them) are loose files under
    // refs/tags or live in packed-refs, both covered above.
}
