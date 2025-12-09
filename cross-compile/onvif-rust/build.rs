//! Build script for ONVIF Rust implementation.
//!
//! This script generates FFI bindings to Anyka SDK headers using bindgen.
//! It handles cross-compilation setup and linking configuration.

use std::env;
use std::path::PathBuf;

fn main() {
    // Declare the use_stubs cfg to avoid warnings
    println!("cargo::rustc-check-cfg=cfg(use_stubs)");

    // Determine if we're cross-compiling
    let target = env::var("TARGET").unwrap_or_else(|_| String::from("native"));
    let is_cross_compile = target.contains("arm") || target.contains("uclibc");

    println!("cargo:rerun-if-changed=build.rs");

    // Only generate FFI bindings when cross-compiling for ARM target
    // For native builds (testing), we use stub implementations
    if is_cross_compile {
        generate_anyka_bindings();
    } else {
        // For native builds, create a marker file indicating stub mode
        println!("cargo:rustc-cfg=use_stubs");
    }
}

/// Generate FFI bindings for Anyka SDK headers.
///
/// We generate a single consolidated bindings file to avoid duplicate type definitions.
fn generate_anyka_bindings() {
    // Use vendor directory with pre-patched headers and libraries
    let vendor_include = PathBuf::from("vendor/include");
    let vendor_lib = PathBuf::from("vendor/lib");

    // Check if vendor headers exist
    if !vendor_include.exists() {
        println!(
            "cargo:warning=Vendor headers not found at {:?}, using stubs",
            vendor_include
        );
        println!("cargo:rustc-cfg=use_stubs");
        return;
    }

    // Create a wrapper header that includes all SDK headers
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let wrapper_path = out_dir.join("anyka_wrapper.h");

    std::fs::write(
        &wrapper_path,
        r#"
// Wrapper header for all Anyka SDK headers
#include "ak_common.h"
#include "ak_vi.h"
#include "ak_venc.h"
#include "ak_ai.h"
#include "ak_aenc.h"
#include "ak_drv_ptz.h"
#include "ak_vpss.h"
#include "ak_drv_irled.h"
"#,
    )
    .expect("Failed to write wrapper header");

    println!(
        "cargo:rerun-if-changed={}",
        vendor_include.join("ak_common.h").display()
    );

    // Get sysroot from environment or use default toolchain path
    let toolchain_base = env::var("TOOLCHAIN_PATH")
        .unwrap_or_else(|_| "/home/kmk/anyka-dev/toolchain/arm-anykav200-crosstool-ng".to_string());
    let sysroot = format!("{}/arm-unknown-linux-uclibcgnueabi/sysroot", toolchain_base);

    let bindings = bindgen::Builder::default()
        .header(wrapper_path.to_string_lossy())
        // Include vendor headers
        .clang_arg(format!("-I{}", vendor_include.display()))
        // ARM cross-compilation settings
        // Use a target triple that clang understands (not the Rust-specific uclibceabi variant)
        .clang_arg("--target=armv5te-unknown-linux-gnueabi")
        .clang_arg(format!("--sysroot={}", sysroot))
        .clang_arg("-march=armv5te")
        .clang_arg("-mfloat-abi=soft")
        .clang_arg("-mtune=arm926ej-s")
        // Include uClibc headers from sysroot
        .clang_arg(format!("-I{}/usr/include", sysroot))
        // Parse settings
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        // Layout tests don't work in cross-compilation
        .layout_tests(false)
        // Generate Rust enums from C enums
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: false,
        })
        // Derive common traits
        .derive_debug(true)
        .derive_default(true)
        .derive_copy(true)
        // Use core types for no_std compatibility
        .use_core()
        // Blocklist problematic types that cause duplicate definitions
        .blocklist_type("max_align_t")
        // Allowlist only the types we actually need
        .allowlist_function("ak_.*")
        .allowlist_type("ak_.*")
        .allowlist_var("AK_.*")
        .generate()
        .unwrap_or_else(|e| {
            panic!("Unable to generate Anyka SDK bindings: {}", e);
        });

    bindings
        .write_to_file(out_dir.join("anyka_bindings.rs"))
        .expect("Couldn't write Anyka bindings");

    // Link against Anyka libraries from vendor directory
    // These match the C ONVIF Makefile's LINKERFLAG
    // Using static linking for all Anyka libraries to avoid uClibc version mismatch
    let lib_path_abs = vendor_lib.canonicalize().unwrap_or(vendor_lib);
    println!("cargo:rustc-link-search=native={}", lib_path_abs.display());

    // Platform libraries (from libplat) - static linking
    println!("cargo:rustc-link-lib=static=plat_common");
    println!("cargo:rustc-link-lib=static=plat_thread");
    println!("cargo:rustc-link-lib=static=plat_vi");
    println!("cargo:rustc-link-lib=static=plat_vpss");
    println!("cargo:rustc-link-lib=static=plat_ipcsrv");
    println!("cargo:rustc-link-lib=static=plat_venc_cb");
    println!("cargo:rustc-link-lib=static=plat_ai");
    println!("cargo:rustc-link-lib=static=plat_drv");

    // MPI libraries (from libmpi) - static linking
    println!("cargo:rustc-link-lib=static=mpi_venc");
    println!("cargo:rustc-link-lib=static=mpi_aenc");
    println!("cargo:rustc-link-lib=static=mpi_aed");

    // SDK component libraries - static linking
    println!("cargo:rustc-link-lib=static=akuio");
    println!("cargo:rustc-link-lib=static=akispsdk");
    println!("cargo:rustc-link-lib=static=akv_encode");
    println!("cargo:rustc-link-lib=static=akstreamenc");
    println!("cargo:rustc-link-lib=static=akaudiocodec");
    println!("cargo:rustc-link-lib=static=akmedialib");
    println!("cargo:rustc-link-lib=static=akae");

    // System libraries - dynamic linking (from toolchain sysroot)
    println!("cargo:rustc-link-lib=pthread");
    println!("cargo:rustc-link-lib=m");
    println!("cargo:rustc-link-lib=dl");
}
