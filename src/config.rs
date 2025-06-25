use anyhow::{anyhow, Context, Result};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use toml::Value;

#[derive(Default, Debug)]
pub struct Config {
    pub qemu: Option<String>,
    pub bios: Option<String>,
    pub run_args: Option<Vec<String>>,
    pub test_args: Option<Vec<String>>,
    pub test_success_exit_code: Option<i32>,
    pub test_timeout: Option<u32>,
    pub copy: Vec<(String, String)>,
}

#[derive(Debug)]
pub struct Profile {
    pub qemu: String,
    pub args: Vec<String>,
    pub test_success_exit_code: i32,
    pub test_timeout: u32,
}

impl Config {
    pub fn read(manifest_path: PathBuf) -> Result<Config> {
        let mut mainfest_context = String::new();
        File::open(&manifest_path)
            .context("Failed to open Cargo.toml")?
            .read_to_string(&mut mainfest_context)
            .context("Failed to read Cargo.toml")?;
        let cargo_toml = mainfest_context
            .parse::<Value>()
            .context("Failed to parse Cargo.toml")?;
        let metadata = match cargo_toml
            .get("package")
            .and_then(|table| table.get("metadata"))
            .and_then(|table| table.get("uefi-runner"))
        {
            None => return Ok(Default::default()),
            Some(meta) => meta
                .as_table()
                .ok_or_else(|| anyhow!("package.metadata.uefi-runner is invalid"))?,
        };
        let mut config: Config = Default::default();
        for (key, value) in metadata {
            match (key.as_str(), value.clone()) {
                ("qemu", Value::String(s)) => config.qemu = Some(s),
                ("bios", Value::String(s)) => config.bios = Some(s),
                ("test-timeout", Value::Integer(i)) => {
                    if i < 0 {
                        return Err(anyhow!("test-timeout must not be negative"));
                    } else {
                        config.test_timeout = Some(i as u32);
                    }
                }
                ("test-success-exit-code", Value::Integer(i)) => {
                    config.test_success_exit_code = Some(i as i32);
                }
                ("run-args", Value::Array(a)) => {
                    let mut args = Vec::new();
                    for v in a {
                        match v {
                            Value::String(s) => args.push(s),
                            _ => return Err(anyhow!("run-args has non string element: {}", v)),
                        }
                    }
                    config.run_args = Some(args);
                }
                ("test-args", Value::Array(a)) => {
                    let mut args = Vec::new();
                    for v in a {
                        match v {
                            Value::String(s) => args.push(s),
                            _ => return Err(anyhow!("test-args has non string element: {}", v)),
                        }
                    }
                    config.test_args = Some(args);
                }
                ("copy", Value::Table(a)) => {
                    let mut args = Vec::new();
                    for (k, v) in a {
                        let v = match v {
                            Value::String(s) => s,
                            _ => return Err(anyhow!("`copy` table has non string element: {}", v)),
                        };
                        args.push((k, v));
                    }
                    config.copy = args;
                }
                (key, value) => {
                    return Err(anyhow!(
                        "unexpect key `{}` with value `{}` in `package.metadata.uefi-runner`",
                        key,
                        value
                    ))
                }
            }
        }
        Ok(config)
    }

    pub fn build_profile(self, is_test: bool, esp: &Path) -> Result<Profile> {
        let qemu = self.qemu.unwrap_or_else(|| "qemu-system-x86_64".into());
        let bios = self.bios.unwrap_or_else(|| "OVMF.fd".into());
        let mut args = if is_test {
            self.test_args.unwrap_or_else(Vec::new)
        } else {
            self.run_args.unwrap_or_else(Vec::new)
        };
        args.push("-bios".into());
        args.push(bios);
        args.push("-drive".into());
        args.push(format!("format=raw,file=fat:rw:{}", esp.display()));
        Ok(Profile {
            qemu,
            args,
            test_success_exit_code: self.test_success_exit_code.unwrap_or(0),
            test_timeout: self.test_timeout.unwrap_or(300),
        })
    }
}
