use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:warning=SCRED-REDACTOR BUILD.RS STARTING");
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let pattern_detector_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("../scred-pattern-detector");
    
    println!("cargo:warning=Building Zig pattern detector...");
    
    // Build Zig library with macOS 11.0 target to match Rust linker
    let zig_build = Command::new("zig")
        .args(["build-lib", "src/lib.zig", "-O", "ReleaseFast", "-target", "aarch64-macos.11.0"])
        .current_dir(&pattern_detector_dir)
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
    let mut lib_path_found: PathBuf = PathBuf::new();
    
    for lib_name in lib_names {
        let lib_path = pattern_detector_dir.join(lib_name);
        if lib_path.exists() {
            let dest = out_dir.join("libscred_pattern_detector.a");
            std::fs::copy(&lib_path, &dest)
                .expect("Failed to copy library");
            println!("cargo:warning=Copied {} to output", lib_name);
            lib_path_found = dest;
            lib_found = true;
            break;
        }
    }
    
    if !lib_found {
        panic!("No Zig library found (liblib.a or lib.a)");
    }
    
    // Link to Zig library using direct path with -force_load
    // Order matters: force_load must come BEFORE linking the library
    println!("cargo:warning=LINKING: {}", lib_path_found.display());
    println!("cargo:rustc-link-arg=-Wl,-force_load,{}", lib_path_found.display());
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=scred_pattern_detector");
    
    println!("cargo:rerun-if-changed={}/src/lib.zig", pattern_detector_dir.display());
}
