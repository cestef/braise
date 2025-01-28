use std::collections::HashMap;

use crate::{BraiseError, Result, file::BraiseFile};

use either::Either;
use log::debug;

// Load environment variables
pub fn load(file: &BraiseFile) -> Result<HashMap<String, String>> {
    let mut env_vars = match &file.dotenv {
        Either::Left(Some(dotenv)) => {
            debug!("Reading dotenv file: {}", dotenv);
            dotenvy::from_filename_iter(dotenv)
                .map_err(BraiseError::from)?
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
