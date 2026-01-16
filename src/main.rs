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

    let metadata = MetadataCommand::new().no_deps().exec().unwrap();
    let config = Config::read(metadata.workspace_root.join_os("Cargo.toml")).unwrap();

    let source_filename = matches.get_one::<String>("file").unwrap();
    let is_test = is_test(source_filename);

    let esp = make_esp(source_filename, &config)?;
    let profile = config.build_profile(is_test, esp.as_path())?;
    let code = run_qemu(is_test, profile)?;

    exit(code)
}

fn is_test(source_filename: &String) -> bool {
    let target_filename_pathbuf: PathBuf = source_filename.into();
    match target_filename_pathbuf.as_path().parent() {
        None => false,
        Some(path) => path.ends_with("deps"),
    }
}

fn make_esp(source_filename: &String, config: &Config) -> Result<PathBuf> {
    let metadata = MetadataCommand::new()
        .no_deps()
        .exec()
        .context("Can't run cargo metadata")?;

    // Create ESP folder
    let esp_root_path_buf: PathBuf = match &config.esp_root {
        None => metadata.target_directory.join("esp").into(),
        Some(path) => {
            if path.is_empty() {
                metadata.target_directory.join("esp").into()
            } else {
                path.into()
            }
        }
    };

    let efi_boot_path = esp_root_path_buf.join("EFI").join("BOOT");
    let file_path_buf: PathBuf = match &config.file_path {
        None => {
            efi_boot_path.join("BOOTX64.EFI").into()
        },
        Some(filename) => {
            if filename.is_empty() {
                efi_boot_path.join("BOOTX64.EFI").into()
            } else {
                esp_root_path_buf.join(filename).into()
            }
        }
    };
    std::fs::create_dir_all(file_path_buf.parent().unwrap())
        .with_context(|| format!("Unable to create {:?} directory", file_path_buf))?;

    std::fs::copy(Path::new(source_filename), file_path_buf)
        .with_context(|| format!("Unable to copy EFI executable {}", source_filename))?;

    for (src, dst) in &config.copy {
        let real_dst = esp_root_path_buf.join(dst);
        std::fs::copy(&src, real_dst)
            .with_context(|| format!("Unable to copy file {} to {}", src, dst))?;
    }
    Ok(esp_root_path_buf)
}

fn run_qemu(is_test: bool, profile: Profile) -> Result<i32> {
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