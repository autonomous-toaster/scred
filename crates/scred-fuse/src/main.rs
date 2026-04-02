use anyhow::{Context, Result};
use fuser::{mount2, spawn_mount2};
use scred_fuse::{BinaryPolicy, MountConfig, RedactingFs};
use std::env;
use std::path::PathBuf;

fn main() -> Result<()> {
    let argv: Vec<String> = env::args().collect();
    if argv.iter().any(|a| a == "-h" || a == "--help") {
        eprintln!("usage: scred-fuse <source-dir> <mountpoint> [--binary=deny|passthrough] [--max-snapshot-size=BYTES] [--chunk-size=BYTES] [--lookahead-size=BYTES] [--redact=SELECTOR]");
        eprintln!("read-only FUSE PoC with in-memory redacted snapshots per open file");
        eprintln!("on macOS, macFUSE must be installed and allowed by the system before mounting works");
        return Ok(());
    }

    let mut args = argv.into_iter().skip(1);
    let source = PathBuf::from(args.next().context("usage: scred-fuse <source-dir> <mountpoint> [--binary=deny|passthrough] [--max-snapshot-size=BYTES]")?);
    let mountpoint = PathBuf::from(args.next().context("usage: scred-fuse <source-dir> <mountpoint> [--binary=deny|passthrough] [--max-snapshot-size=BYTES]")?);

    let mut config = MountConfig {
        source,
        ..MountConfig::default()
    };

    for arg in args {
        if let Some(value) = arg.strip_prefix("--binary=") {
            config.binary_policy = match value {
                "deny" => BinaryPolicy::Deny,
                "passthrough" => BinaryPolicy::Passthrough,
                _ => anyhow::bail!("invalid --binary value: {value}"),
            };
        } else if let Some(value) = arg.strip_prefix("--max-snapshot-size=") {
            config.max_snapshot_size = value.parse()?;
        } else if let Some(value) = arg.strip_prefix("--chunk-size=") {
            config.chunk_size = value.parse()?;
        } else if let Some(value) = arg.strip_prefix("--lookahead-size=") {
            config.lookahead_size = value.parse()?;
        } else if let Some(value) = arg.strip_prefix("--redact=") {
            config.redact_selector = Some(scred_redactor::PatternSelector::from_str(value)
                .map_err(anyhow::Error::msg)?);
        } else {
            anyhow::bail!("unknown argument: {arg}");
        }
    }

    let fs = RedactingFs::new(config.clone())?;
    if env::var("SCRED_FUSE_FOREGROUND").ok().as_deref() == Some("1") {
        mount2(fs, &mountpoint, &config.mount_options())?;
        Ok(())
    } else {
        let session = spawn_mount2(fs, &mountpoint, &config.mount_options())?;
        session.join();
        Ok(())
    }
}
