use clap::{Command, arg};

use crate::utils::version;

// Initialize CLI application and parse arguments
pub fn create() -> Command {
    Command::new(env!("CARGO_PKG_NAME"))
        .allow_external_subcommands(true)
        .version(version())
        .author(clap::crate_authors!())
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(arg!(-i --init <PATH> "Initialize a sample Braise file with the JSON schema"))
        .arg(arg!(-l --list "List all tasks"))
        .arg(arg!(-q --quiet... "Suppress all output"))
        .arg(arg!(-d --debug... "Print debug information"))
        .arg(arg!(-p --parallel "Run tasks in parallel"))
}

pub mod init;
