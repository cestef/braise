use clap::{Command, arg};

pub fn create() -> Command {
    Command::new(env!("CARGO_PKG_NAME"))
        .allow_external_subcommands(true)
        .version(env!("CARGO_PKG_VERSION"))
        .author(clap::crate_authors!())
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(arg!(-f --file <FILE> "Path to the recipe file"))
        .arg(arg!(-d --dry "Dry run mode, does not execute the recipe"))
}
