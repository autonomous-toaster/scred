use scred_fuse::{BinaryPolicy, MountConfig, RedactingFs};
use scred_redactor::PatternSelector;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir() -> PathBuf {
    let mut p = std::env::temp_dir();
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    p.push(format!("scred-fuse-selector-test-{nanos}"));
    fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn selector_none_disables_redaction_for_text_snapshot() {
    let dir = temp_dir();
    let file = dir.join("a.txt");
    let content = b"aws=AKIAIOSFODNN7EXAMPLE\n";
    fs::write(&file, content).unwrap();

    let fs = RedactingFs::new(MountConfig {
        source: dir.clone(),
        binary_policy: BinaryPolicy::Deny,
        redact_selector: Some(PatternSelector::from_str("none").unwrap()),
        ..MountConfig::default()
    })
    .unwrap();

    let out = fs.snapshot_for_path(PathBuf::from("a.txt").as_path()).unwrap();
    assert_eq!(out, content);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn selector_all_enables_redaction_for_text_snapshot() {
    let dir = temp_dir();
    let file = dir.join("a.txt");
    let content = b"aws=AKIAIOSFODNN7EXAMPLE\n";
    fs::write(&file, content).unwrap();

    let fs = RedactingFs::new(MountConfig {
        source: dir.clone(),
        binary_policy: BinaryPolicy::Deny,
        redact_selector: Some(PatternSelector::from_str("all").unwrap()),
        ..MountConfig::default()
    })
    .unwrap();

    let out = fs.snapshot_for_path(PathBuf::from("a.txt").as_path()).unwrap();
    assert_ne!(out, content);
    assert_eq!(out.len(), content.len());

    let _ = fs::remove_dir_all(dir);
}
