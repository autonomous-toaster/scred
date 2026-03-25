use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let pattern_detector_dir = PathBuf::from(&manifest_dir).join("../scred-pattern-detector");
    
    // Build Zig library
    let zig_build = Command::new("zig")
        .args(["build-lib", "src/lib.zig", "-O", "ReleaseFast", "-target", "aarch64-macos.11.0"])
        .current_dir(&pattern_detector_dir)
        .output()
        .expect("Failed to build Zig library");
    
    if !zig_build.status.success() {
        let stderr = String::from_utf8_lossy(&zig_build.stderr);
        panic!("Zig build failed:\n{}", stderr);
    }
    
    // Copy to local crate directory (not build dir)
    let lib_src = pattern_detector_dir.join("liblib.a");
    let lib_dst = PathBuf::from(&manifest_dir).join("libscred_pattern_detector.a");
    
    if lib_src.exists() {
        std::fs::copy(&lib_src, &lib_dst).expect("Failed to copy library");
    } else {
        panic!("liblib.a not found");
    }
    
    // Link the library from source tree (not OUT_DIR)
    println!("cargo:rustc-link-search=native={}", manifest_dir);
    println!("cargo:rustc-link-lib=static=scred_pattern_detector");
    println!("cargo:rustc-link-arg=-Wl,-force_load,{}", lib_dst.display());
}
