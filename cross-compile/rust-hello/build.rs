fn main() {
  // Build the C FFI library
  cc::Build::new()
    .file("src/ffi.c")
    .flag("-march=armv5te")
    .flag("-mfloat-abi=soft")
    .compile("ffi");

  // Build uclibc stubs for glibc-specific functions
  cc::Build::new()
    .file("src/uclibc_stubs.c")
    .flag("-march=armv5te")
    .flag("-mfloat-abi=soft")
    .compile("uclibc_stubs");

  // Rebuild if C source changes
  println!("cargo:rerun-if-changed=src/ffi.c");
  println!("cargo:rerun-if-changed=src/uclibc_stubs.c");
}
