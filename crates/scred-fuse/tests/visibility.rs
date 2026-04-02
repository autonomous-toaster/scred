use scred_fuse::{BinaryPolicy, MountConfig, RedactingFs};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir() -> PathBuf {
    let mut p = std::env::temp_dir();
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    p.push(format!("scred-fuse-visibility-test-{nanos}"));
    fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn denied_binary_is_not_indexed() {
    let dir = temp_dir();
    fs::write(dir.join("text.txt"), b"AWS=AKIAIOSFODNN7EXAMPLE\n").unwrap();
    fs::write(dir.join("blob.bin"), [0_u8, 159, 146, 150]).unwrap();

    let fs = RedactingFs::new(MountConfig {
        source: dir.clone(),
        binary_policy: BinaryPolicy::Deny,
        ..MountConfig::default()
    })
    .unwrap();

    assert!(fs.snapshot_for_path(PathBuf::from("text.txt").as_path()).is_ok());
    assert!(fs.snapshot_for_path(PathBuf::from("blob.bin").as_path()).is_err());

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn passthrough_binary_is_indexed() {
    let dir = temp_dir();
    let blob = [0_u8, 159, 146, 150];
    fs::write(dir.join("blob.bin"), blob).unwrap();

    let fs = RedactingFs::new(MountConfig {
        source: dir.clone(),
        binary_policy: BinaryPolicy::Passthrough,
        ..MountConfig::default()
    })
    .unwrap();

    let out = fs.snapshot_for_path(PathBuf::from("blob.bin").as_path()).unwrap();
    assert_eq!(out, blob);

    let _ = fs::remove_dir_all(dir);
}
