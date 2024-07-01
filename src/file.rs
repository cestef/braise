use std::collections::HashMap;

use crate::{constants::FILE_NAMES, error::BraiseError, task::BraiseTask};
use color_eyre::{eyre::Result, owo_colors::OwoColorize};
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
    for (task, script) in file.tasks {
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

/// A struct representing a Braise file
#[derive(Debug)]
pub struct BraiseFile {
    pub tasks: HashMap<String, BraiseTask>,
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
                        Some((task.clone(), script))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<HashMap<_, _>>();

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
