use color_eyre::{eyre::bail, owo_colors::OwoColorize};
use either::Either;
use log::{debug, trace};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use terminal_size::{terminal_size, Width};

use crate::{
    error::BraiseError,
    file::BraiseFile,
    utils::{get_shell_command, replace_args, replace_env_vars},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct BraiseTask {
    #[serde(alias = "cmd")]
    pub command: Option<String>,
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
    pub workspace: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StringOrBool(#[serde(with = "either::serde_untagged")] pub Either<String, bool>);

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(transparent)]
pub struct BoolOrU8(#[serde(with = "either::serde_untagged")] pub Either<bool, u8>);

// New struct to handle quiet settings
#[derive(Debug, Clone)]
struct QuietSettings {
    task_quiet: Option<BoolOrU8>,
    file_quiet: Either<Option<bool>, Option<u8>>,
    global_quiet: u8,
}

impl QuietSettings {
    fn is_title_quiet(&self) -> bool {
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

    fn is_output_quiet(&self) -> bool {
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

trait CommandResolver {
    fn resolve_command(&self, task_name: &str) -> color_eyre::eyre::Result<String>;
}

impl CommandResolver for BraiseTask {
    fn resolve_command(&self, task_name: &str) -> color_eyre::eyre::Result<String> {
        if self.workspace.unwrap_or(false) {
            let manifest = cargo_toml::Manifest::from_path("Cargo.toml")?;
            if let Some(workspace) = manifest.workspace {
                let members: Vec<String> = workspace
                    .members
                    .iter()
                    .map(|member| {
                        member
                            .split('/')
                            .last()
                            .unwrap_or(member)
                            .split('.')
                            .next()
                            .unwrap_or(member)
                            .to_string()
                    })
                    .collect();
                let member = members.iter().find(|member| *member == task_name);
                if let Some(member) = member {
                    Ok(format!("cargo run --bin {}", member))
                } else {
                    bail!(BraiseError::NoValidTask(task_name.to_string()))
                }
            } else {
                bail!(BraiseError::NoValidTask("No workspace found".to_string()))
            }
        } else {
            self.command.clone().ok_or_else(|| {
                BraiseError::InvalidTask("Need either a command or workspace to run".to_string())
                    .into()
            })
        }
    }
}

pub struct TaskRunner<'a> {
    quiet_settings: QuietSettings,
    file: &'a BraiseFile,
    env_vars: Arc<HashMap<String, String>>,
    ran: Vec<String>,
}

impl<'a> TaskRunner<'a> {
    pub fn new(quiet: u8, file: &'a BraiseFile, env_vars: Arc<HashMap<String, String>>) -> Self {
        Self {
            quiet_settings: QuietSettings {
                task_quiet: None,
                file_quiet: file.quiet.clone(),
                global_quiet: quiet,
            },
            file,
            env_vars,
            ran: Vec::new(),
        }
    }

    fn handle_dependencies(
        &mut self,
        task: &BraiseTask,
        args: &[String],
    ) -> color_eyre::eyre::Result<()> {
        if let Some(deps) = &task.dependencies {
            trace!("run_task: checking dependencies");
            for dep in deps {
                if !self.file.tasks.contains_key(dep) {
                    bail!(BraiseError::InvalidDependency(dep.to_string()));
                }
                if !self.ran.contains(dep) {
                    debug!("Running dependency: {}", dep);
                    let dep_tasks = self.file.tasks.get(dep).unwrap();
                    let dep_task = dep_tasks.iter().find(|task| {
                        task.runs_on
                            .as_ref()
                            .map(|os| {
                                os.iter().any(|os| {
                                    os.to_lowercase() == std::env::consts::OS.to_lowercase()
                                })
                            })
                            .unwrap_or(true)
                    });
                    if let Some(dep_task) = dep_task {
                        self.run_task(dep_task, args, dep)?;
                        self.ran.push(dep.to_string());
                    } else {
                        bail!(BraiseError::NoValidTask(dep.to_string()));
                    }
                }
            }
        }
        Ok(())
    }

    fn print_task_title(&self, command: &str) -> color_eyre::eyre::Result<()> {
        if !self.quiet_settings.is_title_quiet() {
            let title = if command.lines().count() > 1 {
                format!(
                    "{} {}",
                    command.lines().next().unwrap().bold().underline(),
                    "...".dimmed()
                )
            } else {
                command.bold().underline().to_string()
            };

            let terminal_width = terminal_size().map(|(w, _)| w).unwrap_or(Width(80)).0 as usize;

            println!(
                "[{}] {}",
                self.ran.len().dimmed(),
                if title.len() > terminal_width - 4 {
                    title[..terminal_width - 4].to_string()
                } else {
                    title
                }
            );
        }
        Ok(())
    }

    pub fn run_task(
        &mut self,
        task: &BraiseTask,
        args: &[String],
        task_name: &str,
    ) -> color_eyre::eyre::Result<()> {
        trace!("run_task: entering");

        // Set task-specific quiet settings
        self.quiet_settings.task_quiet = task.quiet.clone();

        // Handle dependencies first
        self.handle_dependencies(task, args)?;

        // Resolve command
        let resolved_command = task.resolve_command(task_name)?;
        let (mut command, args) = replace_args(&resolved_command, args)?;
        command = replace_env_vars(&command, &self.env_vars)?;

        // Get shell command
        let shell_command = get_shell_command(task, self.file);
        let (shell, shell_args) = if shell_command.contains(' ') {
            let mut split = shell_command.split_whitespace();
            (split.next().unwrap().to_string(), split.collect::<Vec<_>>())
        } else {
            (shell_command, vec![])
        };

        debug!("Using shell: {}", shell);
        debug!("Shell args: {:#?}", shell_args);

        // Print task title
        self.print_task_title(&command)?;

        // Prepare command
        let to_run = format!("{command} {}", args.join(" "));
        let mut command = std::process::Command::new(shell);
        command
            .args(shell_args)
            .arg(to_run)
            .current_dir(std::env::current_dir()?)
            .envs(self.env_vars.as_ref());

        if self.quiet_settings.is_output_quiet() {
            trace!("run_task: flushing stdout and stderr");
            command.stdout(std::process::Stdio::null());
            command.stderr(std::process::Stdio::null());
        }

        // Execute command
        let status = command.spawn()?.wait()?;

        if !status.success() {
            trace!("run_task: exiting with error");
            bail!(BraiseError::Error(format!(
                "Task {} failed with status code {}",
                task_name,
                status.code().unwrap_or(1)
            )));
        }

        trace!("run_task: exiting");
        Ok(())
    }
}

pub fn run(
    quiet: u8,
    task: &BraiseTask,
    args: &[String],
    file: &BraiseFile,
    env_vars: &HashMap<String, String>,
    task_name: &str,
    ran: Vec<String>,
) -> color_eyre::eyre::Result<()> {
    let env_vars = Arc::new(env_vars.clone());
    let mut runner = TaskRunner::new(quiet, file, env_vars);
    runner.ran = ran;
    runner.run_task(task, args, task_name)
}
