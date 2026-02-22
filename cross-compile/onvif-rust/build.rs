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
}
