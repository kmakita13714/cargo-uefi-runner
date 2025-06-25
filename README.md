# UEFI Runner

A tool for running and testing UEFI projects.

UEFI Runner is modified from [BootUEFI](https://raw.githubusercontent.com/12101111/bootuefi/)

## Overview

UEFI Runner is a tool designed to help developers run and test UEFI (Unified Extensible Firmware Interface) applications written in Rust. It provides an easy way to launch, debug, and test UEFI applications in a development environment.

## Features

- Support for running Rust-based UEFI applications
- Development and testing environment integration
- QEMU integration for virtualized testing

## Requirements

- Rust (latest stable version recommended)
- Cargo
- QEMU (for virtualized testing)
- UEFI target toolchain

## Installation

```shell
cargo install cargo-uefi-runner
```

## Usage

Set `uefi-runner` as a custom runner in `.cargo/config`:

```toml
[build]
target = "x86_64-unknown-uefi"

[target.x86_64-unknown-uefi]
runner = "uefi-runner"
```

You can run your rust UEFI application through `cargo run` or test it throught `cargo test`.

## Configuration

Configuration is done through a through a `[package.metadata.bootuefi]` table in the `Cargo.toml` of your project. The following options are available:

```toml
[package.metadata.uefi-runner]

# The command to run qemu.
# Set this to an absolute path if your qemu is not in your PATH
qemu = "qemu-system-x86_64"

# The Path to UEFI firmware
bios = "OVMF.fd"

# Additional arguments passed to qemu for non-test executables
run-args = []

# Additional arguments passed to qemu for test executables
test-args = []

# An exit code that should be considered as success for test executables
test-success-exit-code = 0

# The timeout for running a test
test-timeout = 300

[package.metadata.uefi-runner.copy]
"boot.conf" = "EFI/BOOT/boot.conf"
"kernel.conf" = "EFI/BOOT/kernel.conf"
(You can specify any number of them.)
```

## License

- MIT license ([LICENSE](LICENSE) or http://opensource.org/licenses/MIT)
