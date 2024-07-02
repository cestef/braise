use std::{collections::HashMap, fmt};

use color_eyre::{eyre::bail, owo_colors::OwoColorize};
use either::Either;
use log::{debug, trace};
use serde::{Deserialize, Serialize};

use crate::{
    error::BraiseError,
    file::BraiseFile,
    utils::{get_shell_command, replace_args, replace_env_vars},
};

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
    pub quiet: Option<BoolOrU8>,
    #[serde(
        alias = "runs-on",
        alias = "runs_on",
        alias = "run-on",
        alias = "run_on",
        alias = "os",
        alias = "platform"
    )]
    pub runs_on: Option<Vec<String>>,
    pub confirm: Option<StringOrBool>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StringOrBool(#[serde(with = "either::serde_untagged")] pub Either<String, bool>);

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BoolOrU8(#[serde(with = "either::serde_untagged")] pub Either<bool, u8>);

impl fmt::Display for BraiseTask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.command)
    }
}

pub fn run_task(
    quiet: u8,
    task: &BraiseTask,
    args: &[String],
    file: &BraiseFile,
    env_vars: &HashMap<String, String>,
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
                let dep_tasks = file.tasks.get(dep).unwrap();
                let dep_task = dep_tasks.iter().find(|task| {
                    task.runs_on
                        .as_ref()
                        .map(|os| {
                            os.iter()
                                .any(|os| os.to_lowercase() == std::env::consts::OS.to_lowercase())
                        })
                        .unwrap_or(true)
                });
                if let Some(dep_task) = dep_task {
                    run_task(quiet, &dep_task, args, file, env_vars, ran.clone())?;
                    ran.push(dep.to_string());
                } else {
                    bail!(BraiseError::NoValidTask(dep.to_string()));
                }
            }
        }
    }

    let (mut command, args) = replace_args(&task.command, args)?;

    command = replace_env_vars(&command, env_vars)?;

    let shell_command = get_shell_command(task, file);

    let (shell, shell_args) = if shell_command.contains(" ") {
        let mut split = shell_command.split_whitespace();
        let shell = split.next().unwrap();
        let args = split.collect::<Vec<_>>();
        (shell.to_string(), args)
    } else {
        (shell_command, vec![])
    };
    debug!("Using shell: {}", shell);
    debug!("Shell args: {:#?}", shell_args);
    let mut shell = std::process::Command::new(shell);

    let to_run = format!("{command} {}", args.join(" "));

    let title_quiet = (quiet > 0)
        || match &task.quiet {
            Some(q) => match q.0 {
                Either::Left(q) => q,
                Either::Right(q) => q > 0,
            },
            _ => false,
        }
        || match file.quiet {
            Either::Left(Some(q)) => q,
            Either::Right(Some(q)) => q > 0,
            _ => false,
        };

    if !title_quiet {
        println!(
            "[{}] {}",
            ran.len().dimmed(),
            (if task.command.lines().count() > 1 {
                format!(
                    "{} {}",
                    task.command.lines().next().unwrap().bold().underline(),
                    "...".dimmed()
                )
            } else {
                task.command.clone().bold().underline().to_string()
            })
            .trim()
        );
    }

    let command = shell
        .args(shell_args)
        .arg(to_run)
        .current_dir(std::env::current_dir()?)
        .envs(env_vars);

    debug!("Running command: {:#?}", command);
    let output_quiet = (quiet > 1)
        || match &task.quiet {
            Some(q) => match q.0 {
                Either::Left(q) => q,
                Either::Right(q) => q > 1,
            },
            _ => false,
        }
        || match file.quiet {
            Either::Left(Some(q)) => q,
            Either::Right(Some(q)) => q > 1,
            _ => false,
        };
    if output_quiet {
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
