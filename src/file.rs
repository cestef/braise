use std::collections::HashMap;

use crate::{constants::FILE_NAMES, error::BraiseError, task::BraiseTask};
use color_eyre::{eyre::Result, owo_colors::OwoColorize};
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

pub fn print_tasks(file: BraiseFile) {
    println!(
        "{}",
        format!("Available tasks in {}:\n", "Braise.toml".bold()).underline()
    );
    for (task, scripts) in file.tasks {
        for script in scripts {
            println!(
                "{}{}",
                task.bold(),
                if let Some(desc) = script.description {
                    format!(": {}", desc.dimmed())
                } else {
                    "".to_string()
                }
            );
        }
    }
}

/// A struct representing a Braise file
#[derive(Debug)]
pub struct BraiseFile {
    pub tasks: HashMap<String, Vec<BraiseTask>>,
    pub shell: Option<String>,
    pub quiet: Option<bool>,
    pub default: Option<String>,
    pub dotenv: Option<String>,
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
                    if let Ok(script) = BraiseTask::deserialize(script.clone()) {
                        Some((task.clone(), vec![script]))
                    } else {
                        debug!("Couldn't parse task: {}", task);
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
        let quiet = value.get("quiet").map(|q| q.as_bool()).flatten();
        let default = value.get("default").map(|d| d.as_str()).flatten();
        let dotenv = value.get("dotenv").map(|d| d.as_str()).flatten();

        Ok(Self {
            tasks,
            shell,
            quiet,
            default: default.map(|d| d.to_string()),
            dotenv: dotenv.map(|d| d.to_string()),
        })
    }
}
