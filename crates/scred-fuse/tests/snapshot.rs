use scred_fuse::{BinaryPolicy, MountConfig, RedactingFs};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir() -> PathBuf {
    let mut p = std::env::temp_dir();
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    p.push(format!("scred-fuse-test-{nanos}"));
    fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn snapshot_redacts_text_file() {
    let dir = temp_dir();
    let file = dir.join("a.txt");
    fs::write(&file, b"token=AKIAIOSFODNN7EXAMPLE\n").unwrap();

    let fs = RedactingFs::new(MountConfig {
        source: dir.clone(),
        binary_policy: BinaryPolicy::Deny,
        ..MountConfig::default()
    })
    .unwrap();

    let out = fs.snapshot_for_path(PathBuf::from("a.txt").as_path()).unwrap();
    assert_eq!(out.len(), b"token=AKIAIOSFODNN7EXAMPLE\n".len());
    assert_ne!(out, b"token=AKIAIOSFODNN7EXAMPLE\n");

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn snapshot_denies_binary_file_by_default() {
    let dir = temp_dir();
    let file = dir.join("b.bin");
    fs::write(&file, [0_u8, 159, 146, 150]).unwrap();

    let fs = RedactingFs::new(MountConfig {
        source: dir.clone(),
        binary_policy: BinaryPolicy::Deny,
        ..MountConfig::default()
    })
    .unwrap();

    assert!(fs.snapshot_for_path(PathBuf::from("b.bin").as_path()).is_err());
    let _ = fs::remove_dir_all(dir);
}
