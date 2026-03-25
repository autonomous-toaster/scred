use std::path::PathBuf;
use std::process::Command;

fn main() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    
    println!("cargo:warning=Building Zig pattern detector...");
    
    // Build Zig library with macOS 11.0 target to match Rust linker
    let zig_build = Command::new("zig")
        .args(["build-lib", "src/lib.zig", "-O", "ReleaseFast", "-target", "aarch64-macos.11.0"])
        .current_dir(&manifest_dir)
        .output()
        .expect("Failed to build Zig library");
    
    if !zig_build.status.success() {
        let stderr = String::from_utf8_lossy(&zig_build.stderr);
        eprintln!("Zig build error:\n{}", stderr);
        panic!("Zig compilation failed");
    }
    
    // Copy library to output directory
    let lib_names = vec!["liblib.a", "lib.a"];
    let mut lib_found = false;
    
    for lib_name in lib_names {
        let lib_path = PathBuf::from(&manifest_dir).join(lib_name);
        if lib_path.exists() {
            let dest = out_dir.join("libscred_pattern_detector.a");
            std::fs::copy(&lib_path, &dest)
                .expect("Failed to copy library");
            println!("cargo:warning=Copied {} to output", lib_name);
            lib_found = true;
            break;
        }
    }
    
    if !lib_found {
        panic!("No Zig library found (liblib.a or lib.a)");
    }
    
    // Link to Zig library - apply to ALL targets (lib, tests, bins)
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=scred_pattern_detector");
    
    // Force whole archive on macOS (needed for FFI exports)
    println!("cargo:rustc-link-arg=-Wl,-force_load,{}/libscred_pattern_detector.a", out_dir.display());
    
    println!("cargo:rerun-if-changed=src/lib.zig");
}
