use crate::{
    bail,
    utils::{replace_args, replace_env_vars, QuietSettings},
    BraiseError, Result,
};
use async_recursion::async_recursion;
use either::Either;
use paris_log::__private_exports_do_not_use::__export_colorize_string as colorize_string;
use paris_log::{debug, trace};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, process::Stdio, sync::Arc};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct BraiseTask {
    #[serde(alias = "cmd")]
    pub command: Option<StringOrVecString>,
    #[serde(alias = "run")]
    pub tasks: Option<Vec<String>>,
    #[serde(alias = "desc")]
    pub description: Option<String>,
    #[serde(alias = "deps", alias = "depends", alias = "depends-on")]
    pub dependencies: Option<Vec<String>>,
    #[serde(alias = "sh")]
    pub shell: Option<String>,
    pub quiet: Option<BoolOrU8>,
    #[serde(alias = "runs-on", alias = "run-on", alias = "os", alias = "platform")]
    pub runs_on: Option<Vec<String>>,
    pub confirm: Option<StringOrBool>,
    pub workspace: Option<bool>,

    #[serde(skip)]
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(transparent)]
pub struct StringOrVecString(
    #[serde(with = "either::serde_untagged")] pub Either<String, Vec<String>>,
);

impl ToString for StringOrVecString {
    fn to_string(&self) -> String {
        match &self.0 {
            Either::Left(s) => s.to_string(),
            Either::Right(v) => v.join(" "),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(transparent)]
pub struct StringOrBool(#[serde(with = "either::serde_untagged")] pub Either<String, bool>);

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(transparent)]
pub struct BoolOrU8(#[serde(with = "either::serde_untagged")] pub Either<bool, u8>);

impl BraiseTask {
    #[async_recursion]
    pub async fn run(
        self,
        env_vars: Arc<HashMap<String, String>>,
        args: Arc<Vec<String>>,
        quiet: &QuietSettings,
        depth_str: Vec<String>,
        task_map: &HashMap<String, Vec<BraiseTask>>,
    ) -> Result<()> {
        // First handle dependencies if any
        if let Some(deps) = &self.dependencies {
            for dep in deps {
                if let Some(tasks) = task_map.get(dep) {
                    for task in tasks {
                        let mut depth_str = depth_str.clone();
                        depth_str.push(dep.clone());
                        task.clone()
                            .run(
                                env_vars.clone(),
                                args.clone(),
                                quiet,
                                depth_str.clone(),
                                task_map,
                            )
                            .await?;
                    }
                } else {
                    bail!(BraiseError::TaskNotFound(dep.clone()));
                }
            }
        }

        // If this is a task group, execute its tasks
        if let Some(tasks) = &self.tasks {
            for task_name in tasks {
                if let Some(tasks) = task_map.get(task_name) {
                    for task in tasks {
                        let mut depth_str = depth_str.clone();
                        depth_str.push(task_name.clone());
                        task.clone()
                            .run(
                                env_vars.clone(),
                                args.clone(),
                                quiet,
                                depth_str.clone(),
                                task_map,
                            )
                            .await?;
                    }
                } else {
                    bail!(BraiseError::TaskNotFound(task_name.clone()));
                }

                return Ok(());
            }
        }

        let command = self
            .command
            .as_ref()
            .ok_or_else(|| {
                BraiseError::InvalidTaskFormat("Missing field: 'command' or 'tasks'".to_string())
            })?
            .to_string();

        let (command, args) = replace_args(&command, &args)?;
        let command = replace_env_vars(&command, &env_vars)?;

        if !quiet.title() {
            println!(
                "{}",
                colorize_string(format!(
                    "[{}]",
                    depth_str
                        .iter()
                        .map(|s| format!("<dimmed>{}</>", s))
                        .collect::<Vec<String>>()
                        .join(" > ")
                ))
            );
        }

        // Get shell command
        let shell_command = self.get_shell_command();
        let (shell, shell_args) = if shell_command.contains(' ') {
            let mut split = shell_command.split_whitespace();
            (split.next().unwrap().to_string(), split.collect::<Vec<_>>())
        } else {
            (shell_command, vec![])
        };

        debug!("Using shell: {}", shell);
        debug!("Shell args: {:#?}", shell_args);

        // Prepare command
        let to_run = format!("{command} {}", args.join(" "));
        let mut tokio_cmd = tokio::process::Command::new(shell);

        tokio_cmd
            .args(shell_args)
            .arg(to_run)
            .current_dir(std::env::current_dir()?)
            .envs(&*env_vars);

        if quiet.output() {
            trace!("run_task: flushing stdout and stderr");
            tokio_cmd.stdout(Stdio::null());
            tokio_cmd.stderr(Stdio::null());
        }

        // Execute command
        let status = tokio_cmd.spawn()?.wait().await?;

        if !status.success() {
            trace!("run_task: exiting with error");
            bail!(BraiseError::TaskError(format!(
                "Task {command} failed with status code {}",
                status.code().unwrap_or(1)
            )));
        }

        Ok(())
    }

    fn get_shell_command(&self) -> String {
        trace!("get_shell_command: entering");
        if let Some(ref shell) = self.shell {
            debug!("Using task shell: {}", shell);
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
}
