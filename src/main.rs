use std::ffi::OsString;

use braise::{
    error::BraiseError,
    file::{find_file, BraiseFile},
    task::run_task,
    utils::{init_panic, version},
};
use clap::{arg, Command};
use color_eyre::{
    eyre::{bail, eyre},
    owo_colors::OwoColorize,
};
use log::{debug, trace};
use toml::Value;

fn main() -> color_eyre::eyre::Result<()> {
    trace!("main: entering");
    init_panic()?;
    pretty_env_logger::init();

    let matches = Command::new(env!("CARGO_PKG_NAME"))
        .allow_external_subcommands(true)
        .version(version())
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(arg!(-l --list "List all tasks"))
        .get_matches();
    debug!("Matches: {:#?}", matches);

    let path = find_file()?;
    debug!("Found file at: {}", path);
    let value = toml::from_str::<Value>(&std::fs::read_to_string(path.clone())?)?;
    debug!("Parsed file: {:#?}", value);
    let file = BraiseFile::from_value(value)?;
    debug!("Parsed braisé file: {:#?}", file);

    if matches.get_flag("list") {
        trace!("main: listing tasks");
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
        trace!("main: exiting from list");
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
        if let Some(default) = &file.default {
            (default.as_str(), vec![])
        } else {
            bail!(BraiseError::NoTask);
        }
    };

    let task = file
        .tasks
        .get(task)
        .ok_or(BraiseError::InvalidTask(task.to_string()))?;
    debug!("Running task: {}", task);
    run_task(task, &args, &file, vec![])?;
    trace!("main: exiting");
    Ok(())
}
