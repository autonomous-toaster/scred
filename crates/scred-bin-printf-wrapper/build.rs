// build.rs - Build script for scred-bin-printf-wrapper
// Compiles C library and links with LD_PRELOAD
// NOTE: Linux/glibc only - macOS/BSD not supported (LD_PRELOAD limitations)

use std::env;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    
    // Only compile C library on Linux
    if target_os != "linux" {
        println!("cargo:warning=scred-bin-printf-wrapper is Linux-only (LD_PRELOAD requirement)");
        println!("cargo:warning=Skipping C compilation on {}", target_os);
        return;
    }
    
    let out_dir = env::var("OUT_DIR").unwrap();
    
    // Compile the C library
    println!("cargo:info=Compiling scred_printf_wrapper.c for Linux");
    
    cc::Build::new()
        .file("src/wrapper.c")
        .pic(true)  // Position-independent code for LD_PRELOAD
        .opt_level(3)  // Optimize for performance
        .warnings(true)
        .warnings_into_errors(false)  // Warnings don't stop build
        .compile("scred_printf_wrapper");
    
    // Tell cargo where to find the library
    println!("cargo:rustc-link-search=native={}", out_dir);
    println!("cargo:rustc-link-lib=static=scred_printf_wrapper");
    
    // Rerun build if C files change
    println!("cargo:rerun-if-changed=src/wrapper.c");
    println!("cargo:rerun-if-changed=build.rs");
}
