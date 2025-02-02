use crate::{
    bail,
    utils::{self, replace_args, replace_env_vars, QuietSettings},
    BraiseError, Result,
};
use async_recursion::async_recursion;
use either::Either;
use paris_log::__private_exports_do_not_use::__export_colorize_string as colorize_string;
use paris_log::{debug, trace};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, process::Stdio, sync::Arc};
use tokio::task::JoinHandle;

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
    #[serde(default)]
    pub workspace: bool,
    #[serde(default)]
    pub watch: Vec<PathBuf>,
    pub cron: Option<String>,

    #[serde(skip)]
    pub name: String,
    #[serde(skip)]
    pub is_dependency: bool,
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
    pub fn is_group(&self) -> bool {
        self.tasks.as_ref().is_some_and(|e| !e.is_empty())
    }

    pub async fn spawn(
        self,
        env_vars: Arc<HashMap<String, String>>,
        args: Arc<Vec<String>>,
        quiet: Arc<QuietSettings>,
        task_map: &HashMap<String, Vec<BraiseTask>>,
    ) -> Result<Vec<JoinHandle<Result<()>>>> {
        let mut handles = vec![];
        let mut depth_str = vec![self.name.clone()];

        self._spawn(
            env_vars,
            args,
            quiet,
            task_map,
            &mut depth_str,
            &mut handles,
        )
        .await?;

        debug!("spawn: {} handles", handles.len());

        Ok(handles)
    }
    #[async_recursion]
    async fn _spawn(
        self,
        env_vars: Arc<HashMap<String, String>>,
        args: Arc<Vec<String>>,
        quiet: Arc<QuietSettings>,
        task_map: &HashMap<String, Vec<BraiseTask>>,
        depth_str: &mut Vec<String>,
        handles: &mut Vec<JoinHandle<Result<()>>>,
    ) -> Result<()> {
        // First handle dependencies if any
        if let Some(deps) = &self.dependencies {
            let mut deps_handles = vec![];
            for dep in deps {
                if let Some(tasks) = utils::get_matching(task_map, dep) {
                    for task in tasks {
                        depth_str.push(dep.clone());

                        if task.is_group() {
                            // TODO: Support group dependencies ?
                            bail!(BraiseError::GroupDependency(task.name.clone()))
                        }

                        task.clone()
                            ._spawn(
                                env_vars.clone(),
                                args.clone(),
                                quiet.clone(),
                                task_map,
                                depth_str,
                                &mut deps_handles,
                            )
                            .await?;

                        depth_str.pop();
                    }
                } else {
                    bail!(BraiseError::TaskNotFound(dep.clone()));
                }
            }
            debug!("_spawn: {} deps", deps_handles.len());

            for handle in deps_handles {
                handle.await.map_err(BraiseError::from)??;
            }
        }
        let depth_str = depth_str
            .iter()
            .map(|s| format!("<dimmed>{}</>", s))
            .collect::<Vec<String>>()
            .join(" > ");

        handles.push(tokio::spawn(async move {
            let command = self
                .command
                .as_ref()
                .ok_or_else(|| BraiseError::MissingField("command".to_string(), self.name.clone()))?
                .to_string();

            let (command, args) = replace_args(&command, &args)?;
            let command = replace_env_vars(&command, &env_vars)?;

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

            if !quiet.title() {
                println!("{}", colorize_string(format!("[{}]", depth_str)));
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
        }));

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
