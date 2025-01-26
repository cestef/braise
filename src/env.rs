use crate::file::BraiseFile;
use color_eyre::{eyre::Context, owo_colors::OwoColorize, Result};
use either::Either;
use log::debug;
use std::collections::HashMap;

// Load environment variables
pub fn load(file: &BraiseFile) -> Result<HashMap<String, String>> {
    let mut env_vars = match &file.dotenv {
        Either::Left(Some(dotenv)) => {
            debug!("Reading dotenv file: {}", dotenv);
            dotenvy::from_filename_iter(dotenv)
                .context(format!("Couldn't read dotenv file: {}", dotenv.bold()))?
                .collect::<Vec<_>>()
        }
        Either::Right(Some(true)) => {
            debug!("Reading dotenv file: .env");
            dotenvy::dotenv_iter()
                .map(|res| res.collect::<Vec<_>>())
                .unwrap_or_default()
        }
        _ => {
            debug!("Not reading dotenv file");
            vec![]
        }
    };

    // Extend with the environment variables from the system
    env_vars.extend(std::env::vars().map(|(key, value)| Ok((key, value))));

    Ok(env_vars
        .iter()
        .filter_map(|res| {
            res.as_ref()
                .ok()
                .map(|(key, value)| (key.to_string(), value.to_string()))
        })
        .collect())
}
