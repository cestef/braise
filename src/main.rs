use std::{collections::HashMap, ffi::OsString};

use braise::{
    error::BraiseError,
    file::{find_file, print_tasks, BraiseFile},
    task::run_task,
    utils::{build_logger, init_panic, version},
};
use clap::{arg, Command};
use color_eyre::{
    eyre::{bail, eyre, Context},
    owo_colors::OwoColorize,
};
use log::{debug, trace};

fn main() -> color_eyre::eyre::Result<()> {
    let mut logger = build_logger();
    trace!("main: entering");
    init_panic()?;

    let matches = Command::new(env!("CARGO_PKG_NAME"))
        .allow_external_subcommands(true)
        .version(version())
        .author(clap::crate_authors!())
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(arg!(-l --list "List all tasks"))
        .arg(arg!(-d --debug... "Print debug information"))
        .get_matches();

    let debug_level = matches.get_count("debug");
    if debug_level == 1 {
        logger.filter_level(log::LevelFilter::Debug);
    } else if debug_level > 1 {
        logger.filter_level(log::LevelFilter::Trace);
    }

    logger.init();

    debug!("Matches: {:#?}", matches);

    let path = find_file()?;
    debug!("Found file at: {}", path);

    let value = toml::from_str::<toml::Value>(&std::fs::read_to_string(path.clone())?)?;
    debug!("Parsed file: {:#?}", value);

    let file = BraiseFile::from_value(value)?;
    debug!("Parsed braisé file: {:#?}", file);

    if matches.get_flag("list") {
        trace!("main: listing tasks");
        print_tasks(file);
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
    let env_vars = if let Some(dotenv) = &file.dotenv {
        if dotenv.is_empty() || dotenv == "false" {
            debug!("Opted-out of dotenv");
            vec![]
        } else {
            debug!("Reading dotenv file: {}", dotenv);
            dotenvy::from_filename_iter(dotenv)
                .context(format!("Couldn't read dotenv file: {}", dotenv.bold()))?
                .collect::<Vec<_>>()
        }
    } else {
        debug!("Reading dotenv file: .env");
        dotenvy::dotenv_iter()
            .map(|res| res.collect::<Vec<_>>())
            .unwrap_or_default()
    };

    let env_vars = env_vars
        .iter()
        .filter_map(|res| {
            if let Ok((key, value)) = res {
                Some((key.to_string(), value.to_string()))
            } else {
                None
            }
        })
        .collect::<HashMap<_, _>>();

    debug!("Env vars: {:#?}", env_vars);

    run_task(task, &args, &file, &env_vars, vec![])?;
    trace!("main: exiting");
    Ok(())
}
