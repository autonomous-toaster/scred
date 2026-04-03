fn main() {
    #[cfg(target_os = "linux")]
    {
        // Export dynamic symbols for LD_PRELOAD to find our hooked functions
        println!("cargo:rustc-link-arg=-Wl,-export-dynamic");
    }
}
