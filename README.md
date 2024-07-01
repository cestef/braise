<p align="center" >
    <img src="assets/header.png" alt="braisé" style="width: 75%" />
</p>

[![Crates.io](https://img.shields.io/crates/v/braise)](https://crates.io/crates/braise)
[![GitHub](https://img.shields.io/github/license/cestef/braise)](LICENSE)
[![Release](https://img.shields.io/github/v/release/cestef/braise)](https://github.com/cestef/braise/releases/latest)

<p align="center">
    <i>Run your tasks like a chef !</i>
</p>

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

> [!NOTE]
> Valid file names are defined [here](src/constants.rs)

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
# Global configuration
shell = "sh -c" # The shell to use (default: $SHELL or "sh -c")
quiet = false # Do not print the command output (default: false)
default = "NAME" # The default task to run (optional)

# Task configuration
[NAME]
command = "echo Hello, World!" # The command to run
description = "A simple task" # The description of the task (optional)
dependencies = ["OTHER_TASK"] # The list of tasks to run before this one (optional)
```

The `shell` and `quiet` fields are global and can be overridden for each task.

> [!NOTE]
> If you are a busy person, you can use the `br` alias instead of `braise`

## License

This project is licensed under the [MIT license](LICENSE) (You can basically do whatever you want with this code)

