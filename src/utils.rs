use std::collections::HashMap;

use const_format::formatcp;
use either::Either;
use paris_log::{
    __private_exports_do_not_use::__export_colorize_string as colorize_string, debug, trace,
};

use crate::{
    bail,
    constants::{ARG_REPLACE_REGEX, ENV_REPLACE_REGEX},
    task::{BoolOrU8, BraiseTask},
    BraiseError, Result,
};

pub static GIT_COMMIT_HASH: &str = env!("_GIT_INFO");

pub fn confirm_action(prompt: &str) -> std::io::Result<bool> {
    println!("{}", prompt);
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    if input.trim().to_lowercase() != "y" {
        println!("Exiting...");
        Ok(false)
    } else {
        Ok(true)
    }
}

pub fn build_logger() -> pretty_env_logger::env_logger::Builder {
    trace!("build_logger: entering");
    let builder = pretty_env_logger::formatted_builder();
    trace!("build_logger: exiting");
    builder
}

pub fn version() -> String {
    colorize_string(VERSION)
}

const VERSION: &str = formatcp!(
    "\
<b>{GIT_COMMIT_HASH}</>

Authors: <dimmed><b>{}</>",
    clap::crate_authors!()
);

pub fn replace_env_vars(input: &str, env_vars: &HashMap<String, String>) -> Result<String> {
    trace!("replace_env_vars: entering");
    let captures = ENV_REPLACE_REGEX.captures_iter(input);
    // Check if there are any missing env vars that don't have a default value
    for capture in captures {
        let var = capture.get(1).unwrap().as_str();
        println!("{:#?}", capture);
        debug!("Checking env var: {}", var);
        if !env_vars.contains_key(var) {
            debug!("Missing env var: {}", var);
            if capture.get(2).is_none() {
                trace!("replace_env_vars: exiting with error");
                bail!(BraiseError::MissingEnvVar(var.to_string()));
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

#[derive(Debug, Clone)]
pub struct QuietSettings {
    pub task_quiet: Option<BoolOrU8>,
    pub file_quiet: Either<Option<bool>, Option<u8>>,
    pub global_quiet: u8,
}

impl QuietSettings {
    pub fn title(&self) -> bool {
        (self.global_quiet > 0)
            || match &self.task_quiet {
                Some(q) => match q.0 {
                    Either::Left(q) => q,
                    Either::Right(q) => q > 0,
                },
                _ => false,
            }
            || match self.file_quiet {
                Either::Left(Some(q)) => q,
                Either::Right(Some(q)) => q > 0,
                _ => false,
            }
    }

    pub fn output(&self) -> bool {
        (self.global_quiet > 1)
            || match &self.task_quiet {
                Some(q) => match q.0 {
                    Either::Left(q) => q,
                    Either::Right(q) => q > 1,
                },
                _ => false,
            }
            || match self.file_quiet {
                Either::Left(Some(q)) => q,
                Either::Right(Some(q)) => q > 1,
                _ => false,
            }
    }
}

// Get the tasks that can be run on this platform (runs_on)
pub fn get_matching<'a>(
    task_map: &'a HashMap<String, Vec<BraiseTask>>,
    task: &'a str,
) -> Option<Vec<&'a BraiseTask>> {
    task_map.get(task).map(|tasks| {
        tasks
            .iter()
            .filter(|task| {
                if let Some(runs_on) = &task.runs_on {
                    runs_on.iter().any(|platform| {
                        let platform = platform.to_lowercase();
                        platform == "all" || platform == std::env::consts::OS.to_lowercase()
                    })
                } else {
                    true
                }
            })
            .collect()
    })
}
