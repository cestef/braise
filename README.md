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
dotenv = ".env" # The path to the .env file (optional)

# Task configuration
[NAME]
command = "echo Hello, World!" # The command to run
description = "A simple task" # The description of the task (optional)
dependencies = ["OTHER_TASK"] # The list of tasks to run before this one (optional)
runs-on = ["linux", "macos"] # The platforms on which the task can run (optional)
```

The `shell` and `quiet` fields are global and can be overridden for each task.

> [!NOTE]
> If you are a busy person, you can use the `br` alias instead of `braise`


### Environment variables

By default `braisé` will look for a `.env` file at the root of your project and load the environment variables from it.

```bash
# .env
HELLO=WORLD
```

```toml
[NAME]
command = "echo $HELLO" # This will let the shell grab the environment variable
# or
command = "echo {env(HELLO)}" # This will expand to "echo WORLD"
```

Default values for environment variables can also be set:

```toml
[NAME]
command = "echo {env(MISSING:default)}" # This will expand to "echo default"
```

### Platform specific commands

You can specify platform specific commands by using the `runs-on` field:

```toml
[[NAME]]
command = "echo Hello, World!"
runs-on = ["windows"]

[[NAME]]
command = "echo Bonjour, Monde!"
runs-on = ["linux", "macos"]
```

Notice the double brackets `[[NAME]]` to define multiple tasks with the same name.

### JSON Schema

A JSON schema is available [here](schema/braise.schema.json) to help you write your `braise.toml` file.

```toml
#:schema https://raw.githubusercontent.com/cestef/braise/main/schema/braise.schema.json

[echo]
command = "echo Hello, World!"
```

You can use the `#:schema` directive to specify the path to the schema file.
## License

This project is licensed under the [MIT license](LICENSE) (You can basically do whatever you want with this code)

