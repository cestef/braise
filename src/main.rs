use std::ffi::OsString;

use braise::{
    error::BraiseError,
    file::{find_file, BraiseFile},
    utils::{init_panic, version},
};
use clap::{arg, Command};
use color_eyre::{
    eyre::{bail, eyre},
    owo_colors::OwoColorize,
};
use lazy_static::lazy_static;
use regex::Regex;
use toml::Value;

lazy_static! {
    static ref REPLACE_REGEX: Regex = Regex::new(r"\{\d\}").unwrap();
}

fn main() -> color_eyre::eyre::Result<()> {
    init_panic()?;

    let matches = Command::new(env!("CARGO_PKG_NAME"))
        .allow_external_subcommands(true)
        .version(version())
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(arg!(-l --list "List all tasks"))
        .get_matches();

    let path = find_file()?;
    let value = toml::from_str::<Value>(&std::fs::read_to_string(path.clone())?)?;
    let file = BraiseFile::from_value(value)?;
    if matches.get_flag("list") {
        println!(
            "{}",
            format!("Available tasks in {}:\n", path.bold()).underline()
        );
        for (task, script) in file.tasks {
            println!(
                "{}: {}",
                task.bold(),
                script.description.unwrap_or("".to_string()).dimmed()
            );
        }
        return Ok(());
    }
    let (task, args) = if let Some((task, matches)) = matches.subcommand() {
        (
            task,
            matches
                .get_many::<OsString>("")
                .ok_or(eyre!("Couldn't parse external args"))?
                .collect::<Vec<_>>()
                .into_iter()
                .map(|s| s.to_string_lossy().to_string())
                .collect::<Vec<_>>(),
        )
    } else {
        bail!(BraiseError::NoTask);
    };

    let task = file
        .tasks
        .get(task)
        .ok_or(BraiseError::InvalidTask(task.to_string()))?;

    let arguments_replace_indexes = REPLACE_REGEX
        .find_iter(&task.command)
        .map(|m| m.as_str().chars().nth(1).unwrap().to_string())
        .collect::<Vec<_>>();

    // Check if the number of arguments provided is the same as the number of indexes
    if arguments_replace_indexes.len() > args.len() {
        bail!(BraiseError::InvalidArgIndex(args.len(), args.len()));
    }

    let command = arguments_replace_indexes
        .iter()
        .fold(task.command.clone(), |acc, index| {
            acc.replacen(
                &format!("{{{}}}", index),
                &args[index.parse::<usize>().unwrap()],
                1,
            )
        });
    // Remove used arguments
    let args = args
        .into_iter()
        .enumerate()
        .filter(|(i, _)| !arguments_replace_indexes.contains(&i.to_string()))
        .map(|(_, arg)| arg)
        .collect::<Vec<_>>();

    let mut shell = std::process::Command::new(if let Some(shell) = file.shell {
        shell.to_string()
    } else if let Some(shell) = std::env::var("SHELL").ok() {
        shell
    } else {
        "sh".to_string()
    });

    let mut child = shell
        .arg("-c")
        .arg(format!("{command} {}", args.join(" ")))
        .spawn()?;

    let status = child.wait()?;

    if !status.success() {
        bail!(BraiseError::Error(format!(
            "Task {} failed with status code {}",
            task,
            status.code().unwrap_or(1)
        )));
    }

    Ok(())
}
