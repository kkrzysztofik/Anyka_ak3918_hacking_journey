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
    println!("cargo:rerun-if-changed=vendor/include");
    println!("cargo:rerun-if-changed=vendor/lib");

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

    // Check if vendor directories exist
    if !vendor_include.exists() {
        println!(
            "cargo:warning=Vendor headers directory not found at {:?}, using stubs",
            vendor_include
        );
        println!("cargo:warning=Run 'scripts/prepare_vendor.sh' to set up vendor files");
        println!("cargo:rustc-cfg=use_stubs");
        return;
    }

    if !vendor_lib.exists() {
        println!(
            "cargo:warning=Vendor libraries directory not found at {:?}, using stubs",
            vendor_lib
        );
        println!("cargo:warning=Run 'scripts/prepare_vendor.sh' to set up vendor files");
        println!("cargo:rustc-cfg=use_stubs");
        return;
    }

    // Verify critical header files (ak_drv_ptz.h omitted: PTZ is implemented in Rust)
    let critical_headers = [
        "ak_common.h",
        "ak_vi.h",
        "ak_venc.h",
        "ak_ai.h",
        "ak_aenc.h",
        "ak_vpss.h",
        "ak_drv_irled.h",
    ];

    let mut missing_headers = Vec::new();
    for header in &critical_headers {
        let header_path = vendor_include.join(header);
        if !header_path.exists() {
            missing_headers.push(header);
        }
    }

    if !missing_headers.is_empty() {
        println!(
            "cargo:warning=Missing critical headers: {:?}, using stubs",
            missing_headers
        );
        println!("cargo:warning=Run 'scripts/prepare_vendor.sh' to set up vendor files");
        println!("cargo:rustc-cfg=use_stubs");
        return;
    }

    // Verify required shared libraries (libre_anyka_app set; .so only, no platform tree)
    let required_libs = [
        "libplat_common.so",
        "libplat_vi.so",
        "libmpi_venc.so",
        "libakuio.so",
    ];
    let mut missing_libs = Vec::new();
    for lib in &required_libs {
        let lib_path = vendor_lib.join(lib);
        let alt = if *lib == "libakuio.so" {
            vendor_lib.join("libakuio.so.3.1.01")
        } else {
            PathBuf::new()
        };
        if !lib_path.exists() && !alt.exists() {
            missing_libs.push(lib);
        }
    }

    if !missing_libs.is_empty() {
        println!(
            "cargo:warning=Missing required .so: {:?}, using stubs",
            missing_libs
        );
        println!(
            "cargo:warning=Run 'scripts/prepare_vendor.sh' (libre_anyka_app lib/ or LIBRE_ANYKA_LIBS_PATH)"
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
// Wrapper header for all Anyka SDK headers (PTZ excluded: native Rust driver)
#include "ak_common.h"
#include "ak_vi.h"
#include "ak_venc.h"
#include "ak_ai.h"
#include "ak_aenc.h"
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

    // Link against Anyka shared libraries from vendor (libre_anyka_app set only; no platform tree)
    // Order matches libre_anyka_app build.sh link line for dependency resolution
    let lib_path_abs = vendor_lib.canonicalize().unwrap_or(vendor_lib);
    println!("cargo:rustc-link-search=native={}", lib_path_abs.display());

    // Allow unresolved symbols between shared libraries; they resolve at runtime
    // via LD_LIBRARY_PATH when all libre_anyka_app .so files are co-located
    println!("cargo:rustc-link-arg=-Wl,--allow-shlib-undefined");

    // Force ALL vendor .so files into the binary's NEEDED list using --no-as-needed.
    //
    // Why: most libre_anyka_app .so files have incomplete DT_NEEDED entries —
    // they only list libc.so.0 and rely on the main binary loading all peers.
    // For example, libplat_vi.so uses akuio_* symbols but doesn't declare
    // libakuio.so as NEEDED. Without --no-as-needed, rustc's default --as-needed
    // drops libraries our Rust code doesn't directly reference, causing runtime
    // "can't resolve symbol" errors on the device.
    //
    // We use rustc-link-arg (not rustc-link-lib) to control exact linker
    // command line ordering: --no-as-needed must precede the -l flags.
    println!("cargo:rustc-link-arg=-Wl,--no-as-needed");

    // Vendor libraries (same .so set as libre_anyka_app)
    let vendor_libs = [
        "akuio",
        "akispsdk",
        "akv_encode",
        "plat_common",
        "plat_thread",
        "plat_vi",
        "plat_vpss",
        "plat_ipcsrv",
        "plat_venc_cb",
        "mpi_venc",
        "akstreamenc",
        "akae",
        "akaudiocodec",
        "akmedia",
        "app_net",
        "app_rtsp",
        "mpi_aed",
        "mpi_aenc",
        "plat_ai",
        "plat_drv",
    ];
    for lib in &vendor_libs {
        println!("cargo:rustc-link-arg=-l{}", lib);
    }

    // Restore --as-needed for system libraries (normal behavior)
    println!("cargo:rustc-link-arg=-Wl,--as-needed");

    // System libraries - dynamic (from toolchain sysroot)
    println!("cargo:rustc-link-lib=pthread");
    println!("cargo:rustc-link-lib=m");
    println!("cargo:rustc-link-lib=dl");
}
