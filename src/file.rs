use either::Either;
use paris_log::{__private_exports_do_not_use::__export_colorize_string as colorize_string, debug};
use serde::Deserialize;

use crate::{
    bail,
    task::{BoolOrU8, BraiseTask},
    utils::QuietSettings,
    BraiseError, Result, FILE_NAMES, TASKS_SEPARATOR,
};

use std::{
    collections::{HashMap, HashSet},
    ffi::OsString,
    path::PathBuf,
    sync::Arc,
};

#[derive(Debug)]
pub struct BraiseFile {
    pub path: PathBuf,
    pub tasks: HashMap<String, Vec<BraiseTask>>,
    pub shell: Option<String>,
    pub quiet: Either<Option<bool>, Option<u8>>,
    pub default: Option<String>,
    pub dotenv: Either<Option<String>, Option<bool>>,
    pub parallel: Option<bool>,
}

impl BraiseFile {
    pub fn new() -> Result<Self> {
        Self::find_path().and_then(Self::load)
    }

    pub fn find_path() -> Result<String> {
        let mut found = None;
        // With and without the .toml extension
        let candidates = FILE_NAMES
            .iter()
            .map(|name| format!("{}.toml", name))
            .chain(FILE_NAMES.iter().map(|name| name.to_string()));
        for candidate in candidates {
            if std::path::Path::new(&candidate).exists() {
                found = Some(candidate);
                break;
            }
        }
        if let Some(found) = found {
            Ok(found)
        } else {
            Err(BraiseError::NoBraiseFileFound.into())
        }
    }

    fn load(path: impl Into<String>) -> Result<Self> {
        let path = path.into();
        debug!("Found file at: {}", path);
        let value = toml::from_str::<toml::Value>(&std::fs::read_to_string(&path)?)?;
        debug!("Parsed file: {:#?}", value);

        let file = Self::from_value(value, &path)?;
        debug!("Parsed braisé file: {:#?}", file);
        Ok(file)
    }

    fn from_value(value: toml::Value, path: impl Into<String>) -> Result<Self> {
        let shell = value
            .get("shell")
            .map(|s| s.as_str().map(|s| s.to_string()))
            .flatten();

        let parsed_tasks = value
            .as_table()
            .ok_or(BraiseError::InvalidFileFormat(
                "File is not a table".to_string(),
            ))?
            .iter()
            .filter_map(|(task, script)| {
                if script.is_table() {
                    let res = BraiseTask::deserialize(script.clone());
                    if let Ok(mut script) = res {
                        script.shell = script.shell.or(shell.clone());
                        script.name = task.clone();
                        Some((task.clone(), vec![script]))
                    } else {
                        debug!("Couldn't parse task <b>{}</>: {}", task, res.unwrap_err());
                        // Try parsing recursively
                        let script = script.as_table().unwrap();
                        let scripts = script
                            .iter()
                            .filter_map(|(task, script)| {
                                if let Ok(mut script) = BraiseTask::deserialize(script.clone()) {
                                    script.shell = script.shell.or(shell.clone());
                                    script.name = task.clone();
                                    Some(script)
                                } else {
                                    debug!("Couldn't parse task: {}", task);
                                    None
                                }
                            })
                            .collect();

                        Some((task.clone(), scripts))
                    }
                } else if script.is_array() {
                    debug!("Task {} is an array", task);
                    // Check if the array is a table
                    let script = script.as_array().unwrap();
                    if script.iter().all(|s| s.is_table()) {
                        let scripts = script
                            .iter()
                            .filter_map(|s| {
                                if let Ok(mut s) = BraiseTask::deserialize(s.clone()) {
                                    s.shell = s.shell.or(shell.clone());
                                    s.name = task.clone();
                                    Some(s)
                                } else {
                                    debug!("Couldn't parse task: {}", task);
                                    None
                                }
                            })
                            .collect();

                        Some((task.clone(), scripts))
                    } else {
                        debug!("Task {} is not a table", task);
                        None
                    }
                } else {
                    debug!("Task {} is not a table", task);
                    None
                }
            })
            .collect::<HashMap<_, Vec<_>>>();

        let quiet = if let Some(quiet) = value.get("quiet") {
            if quiet.is_bool() {
                Either::Left(Some(quiet.as_bool().unwrap()))
            } else if quiet.is_integer() {
                Either::Right(Some(quiet.as_integer().unwrap() as u8))
            } else {
                Either::Left(None)
            }
        } else {
            Either::Left(None)
        };

        let default = value.get("default").map(|d| d.as_str()).flatten();

        let dotenv = if let Some(dotenv) = value.get("dotenv") {
            if dotenv.is_str() {
                Either::Left(Some(dotenv.as_str().unwrap().to_string()))
            } else if dotenv.is_bool() {
                Either::Right(Some(dotenv.as_bool().unwrap()))
            } else {
                Either::Left(None)
            }
        } else {
            Either::Left(None)
        };

        let parallel = value.get("parallel").map(|p| p.as_bool()).flatten();

        const KNOWN_KEYS: [&str; 5] = ["shell", "quiet", "default", "dotenv", "parallel"];

        for key in value
            .as_table()
            .ok_or(BraiseError::InvalidFileFormat(
                "File is not a table".to_string(),
            ))?
            .keys()
        {
            if !KNOWN_KEYS.contains(&key.as_str()) {
                debug!("Unknown key: {}", key);
            }
        }

        Ok(Self {
            tasks: parsed_tasks,
            shell,
            quiet,
            default: default.map(|d| d.to_string()),
            dotenv,
            parallel,
            path: path.into().into(), // path -> String -> PathBuf
        })
    }

    // Get the tasks that can be run on this platform (runs_on)
    pub fn get_tasks(&self, task: &str) -> Option<Vec<&BraiseTask>> {
        self.tasks.get(task).map(|tasks| {
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

    // Parse task input and arguments
    // Returns the task name and the arguments
    pub fn parse_matches(&self, matches: &clap::ArgMatches) -> Result<(String, Vec<String>)> {
        if let Some((input, matches)) = matches.subcommand() {
            Ok((
                input.to_string(),
                matches
                    .get_many::<OsString>("")
                    .ok_or(BraiseError::Error("No arguments provided".to_string()))?
                    .collect::<Vec<_>>()
                    .into_iter()
                    .map(|s| s.to_string_lossy().to_string())
                    .collect(),
            ))
        } else if let Some(ref default) = self.default {
            Ok((default.to_string(), vec![]))
        } else {
            bail!(BraiseError::NoTask);
        }
    }

    pub fn display(&self) {
        let manifest = cargo_toml::Manifest::from_path("Cargo.toml");

        println!(
            "{}",
            colorize_string(format!(
                "<u>Available tasks in <b>{}</>:",
                self.path.to_string_lossy()
            ))
        );

        let maybe_defaults: Option<Vec<_>> = self
            .default
            .as_ref()
            .map(|d| d.split(TASKS_SEPARATOR).map(|d| d.to_string()).collect());
        for (task, scripts) in &self.tasks {
            let is_default = if let Some(ref defaults) = maybe_defaults {
                defaults.contains(&task)
            } else {
                false
            };
            for script in scripts {
                println!(
                    "{}",
                    colorize_string(format!(
                        "<b>{task}</><dimmed>{}{}</>",
                        if let Some(ref desc) = script.description {
                            format!(": {}", desc)
                        } else {
                            "".to_string()
                        },
                        if is_default {
                            " (default)".to_string()
                        } else {
                            "".to_string()
                        }
                    ))
                );
            }
        }
        if let Ok(manifest) = manifest {
            if let Some(workspace) = manifest.workspace {
                println!(
                    "{}",
                    colorize_string("\n<u>Available binaries in <b>Cargo.toml</>:\n")
                );

                for member in workspace.members {
                    let name = member
                        .split('/')
                        .last()
                        .unwrap_or(&member)
                        .split('.')
                        .next()
                        .unwrap_or(&member);
                    println!("{}", colorize_string(format!("<b>{}</>", name)));
                }
            }
        }
    }

    // Build the dependency graph, returns a list of tasks to run in order
    // [task1]
    // command = "echo 1"
    // depends = [task2] // task1.dependencies = ["task2"]
    // [task2]
    // command = "echo 2"
    // depends = [task3] // task2.dependencies = ["task3"]
    // [task3]
    // command = "echo 3" // task3.dependencies = []
    fn build_node(&self, task: &BraiseTask, visited: &mut HashSet<String>) -> Result<BraiseTask> {
        let mut dependencies = vec![];

        // Get the task name by finding it in the tasks HashMap
        let task_name = self
            .tasks
            .iter()
            .find(|(_, tasks)| tasks.iter().any(|t| t == task))
            .map(|(name, _)| name.clone())
            .ok_or_else(|| BraiseError::TaskNotFound("Unknown task".to_string()))?;

        // Check for circular dependencies
        if !visited.insert(task_name.clone()) {
            return Err(BraiseError::CircularDependency(task_name).into());
        }

        if let Some(deps) = &task.dependencies {
            for dep in deps {
                let task = self
                    .tasks
                    .get(dep)
                    .ok_or_else(|| BraiseError::TaskNotFound(dep.clone()))?;

                for task in task {
                    let node = self.build_node(task, visited)?;
                    dependencies.push(node);
                }
            }
        }

        // Remove the task from visited set when we're done with it
        visited.remove(&task_name);

        Ok(task.clone())
    }

    pub fn build_graph(&self, inputs: &[String]) -> Result<Vec<BraiseTask>> {
        let mut graph = vec![];
        let mut visited = HashSet::new();

        for input in inputs.iter() {
            let task = self
                .tasks
                .get(input)
                .ok_or_else(|| BraiseError::TaskNotFound(input.clone()))?;

            for task in task {
                let node = self.build_node(task, &mut visited)?;
                graph.push(node);
            }
        }
        Ok(graph)
    }

    pub fn resolve_globs(&self, inputs: Vec<String>) -> Result<Vec<String>> {
        let mut resolved = vec![];
        for input in inputs {
            let regex = fnmatch_regex::glob_to_regex(&input)?;
            let matches = self
                .tasks
                .keys()
                .filter(|task| regex.is_match(task))
                .map(|task| task.to_string())
                .collect::<Vec<_>>();
            if matches.is_empty() {
                bail!(BraiseError::GlobError(format!(
                    "No tasks found for pattern: {}",
                    input
                )));
            }
            resolved.extend(matches);
        }
        Ok(resolved)
    }

    pub fn quiet_settings(
        &self,
        global_quiet: u8,
        task_quiet: Option<BoolOrU8>,
    ) -> Arc<QuietSettings> {
        Arc::new(QuietSettings {
            file_quiet: self.quiet.clone(),
            task_quiet,
            global_quiet,
        })
    }
}
