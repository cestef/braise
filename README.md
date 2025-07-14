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

- **Types** (revolutionary, I know)
- **Dependencies**
- **Parallel execution**
- **Built-in modules** (`input`, `env`, `git`, `fs` and `cpu` - the essentials)
- **LSP support**
- **Dry run mode** (for the commitment-phobic)

## Quick Start

Install it:

<details>
<summary>with <a href="./docs/static/install.sh"><code>install.sh</code></a></summary>

<p>

```bash
curl -sSL https://braise.cstef.dev/install.sh | INCLUDE_DEV=true bash
```
</p>
</details>

<details>
<summary>with <code>cargo</code></summary>

<p>

```bash
cargo install --git https://github.com/cestef/braise --branch main --bins
```
</p>
</details>

<details>
<summary>with <code>brew</code></summary>

<p>

```bash
brew install cestef/tap/braise
```
</p>
</details>

Create a `Braisefile`:

```
recipe "hello" {
    param name: string = "World"
    print "Hello, ${name}!"
}
```

Run it:

```bash
braise hello --name "Chef"
```

## Why Not Just Use...?

- **Make**: 1976 is calling
- **npm scripts**: JSON isn't a programming language
- **Bash scripts**: Good luck debugging that
- **Justfile**: Not that bad, not "batteries included" (missing LSP, built-in modules, etc.)

## Contributing

Found a bug? Great! [Open an issue](https://github.com/cestef/braise/issues/new). Want to add a feature? Even better! [Submit a PR](https://github.com/cestef/braise/pulls).
