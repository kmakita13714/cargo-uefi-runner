mod config;

use anyhow::{anyhow, Context, Result};
use cargo_metadata::MetadataCommand;
use clap::{Arg, Command};
use config::*;
use std::path::{Path, PathBuf};
use std::process::exit;
use wait_timeout::ChildExt;

fn main() -> Result<()> {
    let matches = Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::new("file")
                .help("Files to run in UEFI mode")
                .required(true)
                .value_name("FILE")
                .index(1)
        )
        .get_matches();

    let uefi_filename = matches.get_one::<String>("file").unwrap();
    let uefi_path: PathBuf = uefi_filename.into();
    let is_test = is_test(uefi_path.as_path());

    let metadata = MetadataCommand::new().no_deps().exec().unwrap();
    let config = Config::read(metadata.workspace_root.join_os("Cargo.toml")).unwrap();

    let esp = make_esp(uefi_path.as_path(), &config.copy)?;
    let profile = config.build_profile(is_test, esp.as_path())?;
    let code = run_qemu(is_test, profile)?;

    exit(code)
}

fn is_test(uefi_path: &Path) -> bool {
    match uefi_path.parent() {
        None => false,
        Some(path) => path.ends_with("deps"),
    }
}

fn make_esp(uefi_path: &Path, copy: &[(String, String)]) -> Result<PathBuf> {
    let target_path = MetadataCommand::new()
        .no_deps()
        .exec()
        .context("Can't run cargo metadata")?
        .target_directory;

    let esp_path = target_path.join("esp");
    let efi_boot_path = esp_path.join("EFI").join("BOOT");
    std::fs::create_dir_all(efi_boot_path.clone())
        .context("Unable to create /EFI/BOOT directory")?;

    let bootx64_path = efi_boot_path.join("BOOTX64.EFI");
    std::fs::copy(uefi_path, bootx64_path)
        .with_context(|| format!("Unable to copy EFI executable {}", uefi_path.display()))?;

    for (src, dst) in copy {
        let real_dst = esp_path.join(dst);
        std::fs::copy(&src, real_dst)
            .with_context(|| format!("Unable to copy file {} to {}", src, dst))?;
    }
    Ok(esp_path.into_std_path_buf())
}

fn run_qemu(is_test: bool, profile: Profile) -> Result<i32> {
    println!("Running: `{} {}`", profile.qemu, profile.args.join(" "));
    let mut cmd = std::process::Command::new(profile.qemu);
    cmd.args(profile.args);
    let exit_code = if is_test {
        let mut child = cmd
            .spawn()
            .with_context(|| format!("Failed to launch QEMU: {:?}", cmd))?;
        let timeout = std::time::Duration::from_secs(profile.test_timeout.into());
        match child
            .wait_timeout(timeout)
            .context("Failed to wait with timeout")?
        {
            None => {
                child.kill().context("Failed to kill QEMU")?;
                child.wait().context("Failed to wait for QEMU process")?;
                return Err(anyhow!("Timed Out"));
            }
            Some(exit_status) => match exit_status.code() {
                Some(code) if code == profile.test_success_exit_code => 0,
                other => other.unwrap_or(1),
            },
        }
    } else {
        let status = cmd
            .status()
            .with_context(|| format!("Failed to execute `{:?}`", cmd))?;
        status.code().unwrap_or(1)
    };
    Ok(exit_code)
}