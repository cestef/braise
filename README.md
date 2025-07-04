<p align="center">
    <img src="assets/banner_dark.gif#gh-dark-mode-only" alt="Braisé Banner" width="100%">
    <img src="assets/banner_light.gif#gh-light-mode-only" alt="Braisé Banner" width="100%">
    <p align="center">
        <a href="https://crates.io/crates/braise"><img src="https://img.shields.io/github/actions/workflow/status/cestef/braise/release.yml?labelColor=%23894733&color=%23894733" alt="Build Status" /></a>
        <a href="https://github.com/cestef/braise/releases"><img src="https://img.shields.io/github/v/release/cestef/braise?labelColor=%23894733&color=%23894733" alt="Release" /></a>
        <a href="LICENSE"><img src="https://img.shields.io/github/license/cestef/braise?labelColor=%23894733&color=%23894733" alt="License" /></a>
    </p>
    <p align="center">
        <b>Run your tasks like a chef! 👨‍🍳</b>
    </p>   
</p>

## What is this?

Yet another task runner, because apparently the world needed one more. But hey, at least this one has types and doesn't make you want to cry when debugging your build scripts.

## Features

- **Actual types** (revolutionary, I know)
- **Dependencies that work**
- **Parallel execution**
- **Built-in modules** (`env`, `git`, `fs` and `cpu` - the essentials)
- **LSP support**
- **Dry run mode** (for the commitment-phobic)

## Quick Start

Install it:

<details>
<summary>with <code>install.sh</code></summary>
</details>
<details>
<summary>with <code>cargo</code></summary>

```bash
cargo install --git https://github.com/cestef/braise --branch dev --bins
```

</details>
<details>
<summary>with <code>brew</code></summary>
</details>

Create a `Braisefile`:

```
recipe "hello" {
    param name: string = "World"
    print "Hello, ${name}!"
}

recipe "build" -> ["test"] {
    run "cargo build --release"
}

recipe "test" {
    run "cargo test"
}
```

Run it:

```bash
braise hello --name "Chef"
braise build
```

## Language Basics

### Parameters (with actual types!)
```
recipe "deploy" {
    param env: ["dev", "prod"] = "dev"    # enum
    param version: string                  # required
    param force: bool = false             # boolean
    param replicas: number = 3            # number
    param services: [string] = []         # array
}
```

### Control Flow
```
recipe "smart-build" {
    if git.is_dirty() {
        print "⚠️  Uncommitted changes detected"
        exit 1
    }
    
    match env.get("CI") {
        "true" => run "cargo build --release",
        _ => run "cargo build"
    }
    
    for service in ["api", "web", "worker"] {
        run "docker build -t ${service} ."
    }
}
```

### Built-in Modules
```
recipe "info" {
    print "Branch: ${git.branch()}"
    print "Commit: ${git.commit_hash_short()}"
    print "CPU cores: ${cpu.count()}"
    print "Home: ${env.HOME}"
    
    if fs.exists("package.json") {
        run "npm install"
    }
}
```

## CLI Usage

```bash
# Basic usage
braise <recipe-name>

# With parameters
braise deploy --env prod --version v1.0.0 --force

# With key=value parameters
braise deploy env=prod version=v1.0.0 force

# Dry run (see what would happen)
braise --dry deploy

# Custom file
braise -f my-recipes.braise build
```

## Why Not Just Use...?

- **Make**: 1976 is calling
- **npm scripts**: JSON isn't a programming language
- **Bash scripts**: Good luck debugging that
- **Justfile**: Actually good, no complaints

## Contributing

Found a bug? Great! [Open an issue](https://github.com/cestef/braise/issues/new). Want to add a feature? Even better! [Submit a PR](https://github.com/cestef/braise/pulls).
