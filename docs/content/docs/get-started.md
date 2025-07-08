+++
weight = 0
title = "Getting Started"
+++

## Installation

Braise can be installed in several ways. Choose the one that suits you best.

### Using [`install.sh`](/install.sh)

```bash, copy
curl -sSL https://braise.cstef.dev/install.sh | INCLUDE_DEV=true bash
```

### Using `cargo`

#### From [crates.io](https://crates.io/crates/braise)

```bash,copy
cargo install braise
# or
cargo binstall braise
```

#### From source

```bash,copy
cargo install --git https://github.com/cestef/braise --branch dev --bins
```

### Using `brew`

```bash,copy
brew install cestef/tap/braise
```

<small> You can also grab the latest release from the [releases page](
    https://github.com/cestef/braise/releases/latest) and install it manually. </small>


## Your first recipe

```braise,copy,name=hello.braise
recipe "hello" {
    param name: string? = "World"
    print "Hello, ${name}!"
}
```