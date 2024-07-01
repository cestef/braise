<p align="center" >
    <img src="assets/header.png" alt="braisé" style="width: 75%" />
</p>

[![Crates.io](https://img.shields.io/crates/v/braise)](https://crates.io/crates/braise)
[![GitHub](https://img.shields.io/github/license/cestef/braise)](LICENSE)
[![Release](https://img.shields.io/github/v/release/cestef/braise)](https://github.com/cestef/braise/releases/latest)

<p align="center">Run your tasks like a chef !</p>

---

<!-- /bʁɛze/ -->

## Installation

### Using homebrew

```bash
brew install cestef/tap/braise
```

### Using cargo

```bash
cargo install braise
```

or with [`cargo-binstall`](https://github.com/cargo-bins/cargo-binstall)

```bash
cargo binstall braise
```

## Usage

All the configuration is done in a `braise.toml` file at the root of your project.

```toml
[NAME]
command = "echo Hello, World!"
```

Then you can run your task with:

```bash
braise NAME
```

The following options are available:

```toml
shell = "sh" # The shell to use (default: $SHELL)
quiet = false # Do not print the command output (default: false)
[NAME]
command = "echo Hello, World!" # The command to run
description = "A simple task" # The description of the task (optional)
dependencies = ["OTHER_TASK"] # The list of tasks to run before this one (optional)
```

The `shell` and `quiet` fields are global and can be overridden for each task.

## License

This project is licensed under the [MIT license](LICENSE).

