use scred_fuse::{BinaryPolicy, MountConfig, RedactingFs};
use std::fs;
use std::os::unix::fs::symlink;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir() -> PathBuf {
    let mut p = std::env::temp_dir();
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    p.push(format!("scred-fuse-cache-test-{nanos}"));
    fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn snapshot_changes_when_file_changes() {
    let dir = temp_dir();
    let file = dir.join("a.txt");
    fs::write(&file, b"AWS=AKIAIOSFODNN7EXAMPLE\n").unwrap();

    let fs = RedactingFs::new(MountConfig {
        source: dir.clone(),
        binary_policy: BinaryPolicy::Deny,
        ..MountConfig::default()
    })
    .unwrap();

    let before = fs.snapshot_for_path(PathBuf::from("a.txt").as_path()).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1100));
    fs::write(&file, b"AWS=AKIAIOSFODNN7EXAMPLE\nNEXT=AKIAIOSFODNN7EXAMPLE\n").unwrap();
    let after = fs.snapshot_for_path(PathBuf::from("a.txt").as_path()).unwrap();

    assert_ne!(before, after);
    assert!(after.len() > before.len());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn symlink_is_not_indexed() {
    let dir = temp_dir();
    fs::write(dir.join("real.txt"), b"hello\n").unwrap();
    symlink(dir.join("real.txt"), dir.join("link.txt")).unwrap();

    let fs = RedactingFs::new(MountConfig {
        source: dir.clone(),
        binary_policy: BinaryPolicy::Deny,
        ..MountConfig::default()
    })
    .unwrap();

    assert!(fs.snapshot_for_path(PathBuf::from("real.txt").as_path()).is_ok());
    assert!(fs.snapshot_for_path(PathBuf::from("link.txt").as_path()).is_err());
    let _ = fs::remove_dir_all(dir);
}
