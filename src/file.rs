use std::{collections::HashMap, fmt};

use crate::{constants::FILE_NAMES, error::BraiseError};
use color_eyre::eyre::Result;
use serde::{Deserialize, Serialize};

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
}

impl fmt::Display for BraiseTask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.command)
    }
}

/// A struct representing a Braise file
#[derive(Debug)]
pub struct BraiseFile {
    pub tasks: HashMap<String, BraiseTask>,
    pub shell: Option<String>,
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

        Ok(Self { tasks, shell })
    }
}
