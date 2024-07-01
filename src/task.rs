use std::fmt;

use color_eyre::{eyre::bail, owo_colors::OwoColorize};
use log::{debug, trace};
use serde::{Deserialize, Serialize};

use crate::{constants::REPLACE_REGEX, error::BraiseError, file::BraiseFile};

/// A struct representing a Braise task
/// ```toml
/// [task]
/// cmd = "echo Hello, World!"
/// desc = "Prints 'Hello, World!'"
/// ```
#[derive(Debug, Serialize, Deserialize)]
pub struct BraiseTask {
    #[serde(alias = "cmd")]
    pub command: String,
    #[serde(alias = "desc")]
    pub description: Option<String>,
    #[serde(alias = "deps", alias = "depends", alias = "depends_on")]
    pub dependencies: Option<Vec<String>>,
    #[serde(alias = "sh")]
    pub shell: Option<String>,
    #[serde(alias = "q")]
    pub quiet: Option<bool>,
}

impl fmt::Display for BraiseTask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.command)
    }
}

pub fn run_task(
    task: &BraiseTask,
    args: &[String],
    file: &BraiseFile,
    mut ran: Vec<String>,
) -> color_eyre::eyre::Result<()> {
    trace!("run_task: entering");
    if let Some(deps) = &task.dependencies {
        trace!("run_task: checking dependencies");
        for dep in deps {
            if !file.tasks.contains_key(dep) {
                trace!("run_task: exiting with error");
                bail!(BraiseError::InvalidDependency(dep.to_string()));
            }
            if !ran.contains(dep) {
                debug!("Running dependency: {}", dep);
                trace!("run_task: recursing");
                run_task(&file.tasks[dep], args, file, ran.clone())?;
                ran.push(dep.to_string());
            }
        }
    }

    let arguments_replace_indexes = REPLACE_REGEX
        .find_iter(&task.command)
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
    debug!("Max index: {:#?}\nArgs len: {:#?}", max_index, args.len());
    if let Some(max_index) = max_index {
        if max_index >= &args.len() {
            trace!("run_task: exiting with error");
            bail!(BraiseError::InvalidArgIndex(*max_index, args.len()));
        }
    }

    let command = arguments_replace_indexes
        .iter()
        .fold(task.command.clone(), |acc, index| {
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

    let shell = if let Some(ref shell) = task.shell {
        debug!("Using task shell: {}", shell);
        shell.to_string()
    } else if let Some(ref shell) = file.shell {
        debug!("Using file shell: {}", shell);
        shell.to_string()
    } else if let Some(shell) = std::env::var("SHELL").ok() {
        debug!("Using SHELL env var: {}", shell);
        shell
    } else {
        debug!("No shell found, exiting");
        trace!("run_task: exiting with error");
        bail!(BraiseError::NoShell);
    };
    let mut shell = std::process::Command::new(shell);

    let to_run = format!("{command} {}", args.join(""));

    let quiet = task.quiet.unwrap_or(file.quiet.unwrap_or(false));
    if !quiet {
        println!(
            "[{}] {}",
            ran.len().dimmed(),
            to_run.trim().bold().underline()
        );
    }

    let command = shell.arg("-c").arg(to_run);

    if quiet {
        trace!("run_task: flushing stdout and stderr");
        command.stdout(std::process::Stdio::null());
        command.stderr(std::process::Stdio::null());
    }

    let mut child = command.spawn()?;

    let status = child.wait()?;

    if !status.success() {
        trace!("run_task: exiting with error");
        bail!(BraiseError::Error(format!(
            "Task {} failed with status code {}",
            task,
            status.code().unwrap_or(1)
        )));
    }

    trace!("run_task: exiting");
    Ok(())
}
