use scred_fuse::{BinaryPolicy, MountConfig, RedactingFs};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir() -> PathBuf {
    let mut p = std::env::temp_dir();
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    p.push(format!("scred-fuse-layout-test-{nanos}"));
    fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn nested_paths_are_indexed_and_snapshot_is_stable() {
    let dir = temp_dir();
    fs::create_dir_all(dir.join("nested/deeper")).unwrap();
    let content = b"prefix\nAWS=AKIAIOSFODNN7EXAMPLE\nsuffix\n";
    fs::write(dir.join("nested/deeper/secret.txt"), content).unwrap();

    let fs = RedactingFs::new(MountConfig {
        source: dir.clone(),
        binary_policy: BinaryPolicy::Deny,
        ..MountConfig::default()
    })
    .unwrap();

    let snap1 = fs
        .snapshot_for_path(PathBuf::from("nested/deeper/secret.txt").as_path())
        .unwrap();
    let snap2 = fs
        .snapshot_for_path(PathBuf::from("nested/deeper/secret.txt").as_path())
        .unwrap();

    assert_eq!(snap1, snap2);
    assert_eq!(snap1.len(), content.len());

    let a = &snap1[0..10.min(snap1.len())];
    let b = &snap1[5..15.min(snap1.len())];
    assert_eq!(a, &snap2[0..10.min(snap2.len())]);
    assert_eq!(b, &snap2[5..15.min(snap2.len())]);

    let _ = fs::remove_dir_all(dir);
}
