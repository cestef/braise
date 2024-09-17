use std::collections::HashMap;

use crate::{constants::FILE_NAMES, error::BraiseError, task::BraiseTask};
use color_eyre::{eyre::Result, owo_colors::OwoColorize};
use either::Either;
use log::debug;
use serde::Deserialize;

pub fn find_file() -> Result<String> {
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

pub fn print_tasks(file: &BraiseFile) {
    let manifest = cargo_toml::Manifest::from_path("Cargo.toml");
    println!(
        "{}",
        format!("Available tasks in {}:\n", "Braise.toml".bold()).underline()
    );
    for (task, scripts) in &file.tasks {
        let is_default = if let Some(ref default) = file.default {
            default == task
        } else {
            false
        };
        for script in scripts {
            println!(
                "{}{}{}",
                task.bold(),
                if let Some(ref desc) = script.description {
                    format!(": {}", desc.dimmed())
                } else {
                    "".to_string()
                },
                if is_default {
                    " (default)".dimmed().to_string()
                } else {
                    "".to_string()
                }
            );
        }
    }
    if let Ok(manifest) = manifest {
        if let Some(workspace) = manifest.workspace {
            println!(
                "{}",
                format!("\nAvailable binaries in {}:\n", "Cargo.toml".bold()).underline()
            );
            for member in workspace.members {
                let name = member
                    .split('/')
                    .last()
                    .unwrap_or(&member)
                    .split('.')
                    .next()
                    .unwrap_or(&member);
                println!("{}", name.bold());
            }
        }
    }
}

/// A struct representing a Braise file
#[derive(Debug)]
pub struct BraiseFile {
    pub tasks: HashMap<String, Vec<BraiseTask>>,
    pub shell: Option<String>,
    pub quiet: Either<Option<bool>, Option<u8>>,
    pub default: Option<String>,
    pub dotenv: Either<Option<String>, Option<bool>>,
    pub parallel: Option<bool>,
}

impl BraiseFile {
    pub fn from_value(value: toml::Value) -> Result<Self> {
        let tasks = value
            .as_table()
            .ok_or(BraiseError::InvalidFileFormat(
                "File is not a table".to_string(),
            ))?
            .iter()
            .filter_map(|(task, script)| {
                if script.is_table() {
                    let res = BraiseTask::deserialize(script.clone());
                    if let Ok(script) = res {
                        Some((task.clone(), vec![script]))
                    } else {
                        debug!("Couldn't parse task {}: {}", task.bold(), res.unwrap_err());
                        None
                    }
                } else if script.is_array() {
                    debug!("Task {} is an array", task);
                    // Check if the array is a table
                    let script = script.as_array().unwrap();
                    if script.iter().all(|s| s.is_table()) {
                        let scripts = script
                            .iter()
                            .filter_map(|s| {
                                if let Ok(s) = BraiseTask::deserialize(s.clone()) {
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

        let shell = value
            .get("shell")
            .map(|s| s.as_str().map(|s| s.to_string()))
            .flatten();

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

        Ok(Self {
            tasks,
            shell,
            quiet,
            default: default.map(|d| d.to_string()),
            dotenv,
            parallel,
        })
    }
}
