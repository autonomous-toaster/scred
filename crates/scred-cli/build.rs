use std::path::PathBuf;

fn main() {
    // Ensure Zig library is built first (by depending on scred-redactor)
    // which handles the build.rs for Zig compilation.
    
    // Now link it to the BINARY
    let redactor_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("../scred-redactor");
    let lib_path = redactor_dir.join("libscred_pattern_detector.a");
    
    if !lib_path.exists() {
        panic!("Library not found at {}", lib_path.display());
    }
    
    // CRITICAL: These linker args go to the BINARY link step, not library link
    println!("cargo:rustc-link-search=native={}", redactor_dir.display());
    println!("cargo:rustc-link-lib=static=scred_pattern_detector");
    // macOS specific: force_load pulls in all symbols
    println!("cargo:rustc-link-arg=-Wl,-force_load,{}", lib_path.display());
}
