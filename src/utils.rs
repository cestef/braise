use std::collections::HashMap;

use color_eyre::{
    eyre::{bail, Result},
    owo_colors::OwoColorize,
};
use log::{debug, trace};

use crate::{
    constants::{ARG_REPLACE_REGEX, ENV_REPLACE_REGEX},
    error::BraiseError,
    file::BraiseFile,
    task::BraiseTask,
};

pub static GIT_COMMIT_HASH: &str = env!("_GIT_INFO");

pub fn init_panic() -> Result<()> {
    trace!("init_panic: entering");
    let (panic_hook, eyre_hook) = color_eyre::config::HookBuilder::default()
        .panic_section(format!(
            "This is a bug. Consider reporting it at {}",
            env!("CARGO_PKG_REPOSITORY")
        ))
        .capture_span_trace_by_default(false)
        .display_location_section(false)
        .display_env_section(false)
        .into_hooks();
    trace!("init_panic: installing eyre hook");
    eyre_hook.install()?;
    trace!("init_panic: installing panic hook");
    std::panic::set_hook(Box::new(move |panic_info| {
        #[cfg(not(debug_assertions))]
        {
            use human_panic::{handle_dump, print_msg, Metadata};
            trace!("init_panic: human-panic hook");
            let meta = Metadata::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
                .authors(env!("CARGO_PKG_AUTHORS").replace(':', ", "))
                .homepage(env!("CARGO_PKG_HOMEPAGE"));

            let file_path = handle_dump(&meta, panic_info);
            // prints human-panic message
            print_msg(file_path, &meta)
                .expect("human-panic: printing error message to console failed");
            eprintln!("{}", panic_hook.panic_report(panic_info)); // prints color-eyre stack trace to stderr
        }
        let msg = format!("{}", panic_hook.panic_report(panic_info));
        println!("Error: {}", strip_ansi_escapes::strip_str(msg));

        #[cfg(debug_assertions)]
        {
            trace!("init_panic: better-panic hook");
            // Better Panic stacktrace that is only enabled when debugging.
            better_panic::Settings::auto()
                .most_recent_first(false)
                .lineno_suffix(true)
                .verbosity(better_panic::Verbosity::Full)
                .create_panic_handler()(panic_info);
        }

        std::process::exit(1);
    }));
    trace!("init_panic: hooks installed");
    trace!("init_panic: exiting");
    Ok(())
}

pub fn build_logger() -> pretty_env_logger::env_logger::Builder {
    trace!("build_logger: entering");
    let builder = pretty_env_logger::formatted_builder();
    trace!("build_logger: exiting");
    builder
}

pub fn version() -> String {
    trace!("version: entering");
    let author = clap::crate_authors!();
    debug!("version: raw_author: {}", author);
    let author = author.replace(':', ", ");
    let hash = GIT_COMMIT_HASH.bold();
    trace!("version: exiting");
    format!(
        "\
{hash}

Authors: {}",
        author.dimmed().bold(),
    )
}

pub fn replace_env_vars(input: &str, env_vars: &HashMap<String, String>) -> Result<String> {
    trace!("replace_env_vars: entering");
    let captures = ENV_REPLACE_REGEX.captures_iter(input);
    // Check if there are any missing env vars that don't have a default value
    for capture in captures {
        let var = capture.get(1).unwrap().as_str();
        debug!("Checking env var: {}", var);
        if !env_vars.contains_key(var) {
            debug!("Missing env var: {}", var);
            if capture.get(2).is_none() {
                trace!("replace_env_vars: exiting with error");
                bail!(BraiseError::Error(format!(
                    "Missing environment variable: {}",
                    var
                )));
            }
        }
    }
    let replaced = ENV_REPLACE_REGEX.replace_all(input, |caps: &regex::Captures| {
        let var = caps.get(1).unwrap().as_str();
        let default = caps.get(2).map(|m| m.as_str()).unwrap_or_default();
        env_vars
            .get(var)
            .map(|e| e.to_string())
            .unwrap_or(default.to_string())
    });
    trace!("replace_env_vars: exiting");
    Ok(replaced.to_string())
}

pub fn replace_args(input: &str, args: &[String]) -> Result<(String, Vec<String>)> {
    trace!("replace_args: entering");
    let arguments_replace_indexes = ARG_REPLACE_REGEX
        .find_iter(input)
        .map(|m| {
            m.as_str()
                .chars()
                .nth(1)
                .unwrap()
                .to_string()
                .parse::<usize>()
                .unwrap()
        })
        .collect::<Vec<_>>();

    // Check if the biggest index is bigger than the number of arguments
    let max_index = arguments_replace_indexes.iter().max();

    debug!("Max index: {:#?}", max_index);
    debug!("Args len: {:#?}", args.len());
    if let Some(max_index) = max_index {
        if max_index >= &args.len() {
            trace!("run_task: exiting with error");
            bail!(BraiseError::InvalidArgIndex(*max_index, args.len()));
        }
    }

    let command = arguments_replace_indexes
        .iter()
        .fold(input.to_string(), |acc, index| {
            acc.replacen(&format!("{{{}}}", index), &args[*index], 1)
        });
    debug!("Command after replacement: {}", command);
    // Remove used arguments
    let args = args
        .into_iter()
        .enumerate()
        .filter(|(i, _)| !arguments_replace_indexes.contains(i))
        .map(|(_, arg)| arg.to_string())
        .collect::<Vec<_>>();
    debug!("Arguments after replacement: {:#?}", args);
    trace!("replace_args: exiting");
    Ok((command, args))
}

pub fn get_shell_command(task: &BraiseTask, file: &BraiseFile) -> String {
    trace!("get_shell_command: entering");
    if let Some(ref shell) = task.shell {
        debug!("Using task shell: {}", shell);
        shell.to_string()
    } else if let Some(ref shell) = file.shell {
        debug!("Using file shell: {}", shell);
        shell.to_string()
    } else if let Some(shell) = std::env::var("SHELL").ok() {
        debug!("Using SHELL env var: {}", shell);
        match shell.as_str() {
            "powershell" => format!("{} -Command", shell),
            "cmd" => format!("{} /c", shell),
            _ => format!("{} -c", shell),
        }
    } else {
        match std::env::consts::OS {
            "windows" => {
                debug!("Using default shell for Windows: powershell");
                "powershell -Command".to_string()
            }
            _ => {
                debug!("Using default shell for Unix: sh");
                "sh -c".to_string()
            }
        }
    }
}
