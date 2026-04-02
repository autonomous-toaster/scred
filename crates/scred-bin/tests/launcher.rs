use std::path::PathBuf;

#[test]
fn preload_path_can_be_overridden() {
    let p = PathBuf::from("/tmp/libscred_bin_preload.so");
    std::env::set_var("SCRED_BIN_PRELOAD_LIB", &p);
    let got = std::env::var_os("SCRED_BIN_PRELOAD_LIB").unwrap();
    assert_eq!(PathBuf::from(got), p);
}
