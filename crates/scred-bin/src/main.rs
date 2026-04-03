use anyhow::{bail, Context, Result};
use std::env;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

fn main() -> Result<ExitCode> {
    if env::args().any(|a| a == "-h" || a == "--help") {
        eprintln!("usage: scred-bin [OPTIONS] [--] <program> [args...]");
        eprintln!();
        eprintln!("Output redaction options:");
        eprintln!("  --stdout          Hook stdout (default: yes)");
        eprintln!("  --no-stdout       Don't hook stdout");
        eprintln!("  --stderr          Hook stderr (default: yes)");
        eprintln!("  --no-stderr       Don't hook stderr");
        eprintln!("  --network         Hook network sockets (default: no)");
        eprintln!();
        eprintln!("Redaction configuration:");
        eprintln!("  --lookahead <N>   Buffer size for pattern matching (default: 4096 bytes)");
        eprintln!("  --redact <TYPES>  Pattern types to redact (passed to scred-detector)");
        eprintln!();
        eprintln!("Debugging:");
        eprintln!("  --debug-hooks     Log hook invocations to stderr");
        eprintln!();
        eprintln!("Notes:");
        eprintln!("  - Linux/glibc-first LD_PRELOAD output redaction");
        eprintln!("  - musl/alpine is experimental");
        eprintln!("  - macOS/BSD not supported (LD_PRELOAD limitation)");
        return Ok(ExitCode::SUCCESS);
    }

    run_platform()
}

#[cfg(not(target_os = "linux"))]
fn run_platform() -> Result<ExitCode> {
    eprintln!("scred-bin currently supports Linux/glibc only; macOS is not supported in this PoC");
    eprintln!("reason: this implementation relies on LD_PRELOAD-style interposition");
    Ok(ExitCode::FAILURE)
}

#[cfg(target_os = "linux")]
fn run_platform() -> Result<ExitCode> {
    let mut args = env::args().skip(1);
    let mut hook_stdout = true;
    let mut hook_stderr = true;
    let mut hook_network = false;
    let mut debug_hooks = false;
    let mut lookahead: Option<usize> = None;
    let mut redact_patterns: Option<String> = None;
    let mut passthrough = Vec::new();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--stdout" => hook_stdout = true,
            "--stderr" => hook_stderr = true,
            "--no-stdout" => hook_stdout = false,
            "--no-stderr" => hook_stderr = false,
            "--network" => hook_network = true,
            "--debug-hooks" => debug_hooks = true,
            "--lookahead" => {
                let val = args.next()
                    .ok_or_else(|| anyhow::anyhow!("--lookahead requires a numeric argument"))?
                    .parse::<usize>()
                    .context("--lookahead value must be a valid number")?;
                lookahead = Some(val);
            }
            "--redact" => {
                redact_patterns = args.next();
            }
            "--" => {
                passthrough.extend(args);
                break;
            }
            _ if arg.starts_with('-') && passthrough.is_empty() => passthrough.push(arg),
            _ => {
                passthrough.push(arg);
                passthrough.extend(args);
                break;
            }
        }
    }

    let program = passthrough.first().cloned().context(
        "usage: scred-bin [OPTIONS] [--] <program> [args...]",
    )?;
    let child_args: Vec<String> = passthrough.into_iter().skip(1).collect();

    if matches!(program.as_str(), "echo" | "printf" | "test" | "[") {
        eprintln!("warning: '{program}' is often a shell builtin; prefer an external binary or run inside Linux/Podman for this PoC");
    }

    let preload = find_preload_library()?;
    let current_ld_preload = env::var_os("LD_PRELOAD");
    let merged_ld_preload = match current_ld_preload {
        Some(existing) if !existing.is_empty() => {
            let mut s = preload.to_string_lossy().to_string();
            s.push(':');
            s.push_str(&existing.to_string_lossy());
            s
        }
        _ => preload.to_string_lossy().to_string(),
    };

    let mut cmd = Command::new(&program);
    cmd.args(&child_args)
        .env("LD_PRELOAD", merged_ld_preload)
        .env("SCRED_BIN_ACTIVE", "1")
        .env("SCRED_BIN_HOOK_STDOUT", if hook_stdout { "1" } else { "0" })
        .env("SCRED_BIN_HOOK_STDERR", if hook_stderr { "1" } else { "0" })
        .env("SCRED_BIN_HOOK_NETWORK", if hook_network { "1" } else { "0" })
        .env("SCRED_BIN_DEBUG_HOOKS", if debug_hooks { "1" } else { "0" });

    // Pass through optional configuration
    if let Some(la) = lookahead {
        cmd.env("SCRED_LOOKAHEAD", la.to_string());
    }
    if let Some(patterns) = redact_patterns {
        cmd.env("SCRED_REDACT_PATTERNS", patterns);
    }

    let status = cmd.status()
        .with_context(|| format!("failed to spawn program: {program}"))?;

    Ok(match status.code() {
        Some(code) => ExitCode::from(code as u8),
        None => ExitCode::FAILURE,
    })
}

fn find_preload_library() -> Result<PathBuf> {
    if let Some(path) = env::var_os("SCRED_BIN_PRELOAD_LIB") {
        return Ok(PathBuf::from(path));
    }

    let exe = env::current_exe()?;
    let mut candidates = Vec::new();

    if let Some(dir) = exe.parent() {
        candidates.push(dir.join("libscred_bin_preload.so"));
        candidates.push(dir.join("../lib/libscred_bin_preload.so"));
        candidates.push(dir.join("../lib64/libscred_bin_preload.so"));
    }
    candidates.push(PathBuf::from("target/debug/libscred_bin_preload.so"));
    candidates.push(PathBuf::from("target/release/libscred_bin_preload.so"));

    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    bail!("could not locate libscred_bin_preload.so; set SCRED_BIN_PRELOAD_LIB")
}
